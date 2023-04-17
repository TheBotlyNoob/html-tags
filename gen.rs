#!/usr/bin/env rust-script
//! This is a regular crate doc comment, but it also contains a partial
//! Cargo manifest.  Note the use of a *fenced* code block, and the
//! `cargo` "language".
//!
//! ```cargo
//! [dependencies]
//! heck = "0.4.1"
//! scraper = "0.16.0"
//! ureq = "2.6.2"
//! itertools = "0.10.5"
//! ```

use heck::{AsKebabCase, ToSnakeCase, ToUpperCamelCase};
use itertools::Itertools;
use scraper::{Element, ElementRef, Html, Selector};
use std::{collections::BTreeMap, io::Write};

fn main() {
    let agent = ureq::agent();

    let resp = agent
        .get("https://developer.mozilla.org/en-US/docs/Web/HTML/Element")
        .call()
        .unwrap();
    let html = resp.into_string().unwrap();
    let document = Html::parse_document(&html);
    let selector =
        Selector::parse("td:first-child > a[href^='/en-US/docs/Web/HTML/Element/']:only-child")
            .unwrap();

    let mut elems = Vec::new();

    let global_attrs = BTreeMap::from_iter(get_global_attrs(false));
    let owned_global_attrs = BTreeMap::from_iter(get_global_attrs(true));

    let mut buf = String::from(
        "// generated by gen.rs + rustfmt - not in a build.rs because HTML tags don't change too often
        //! An auto-generated crate containing all HTML tags and their attributes.
        //! This crate is generated from the [MDN HTML element reference](https://developer.mozilla.org/en-US/docs/Web/HTML/Element).
        //! 
        //! The `<element>Owned` variants are the same as the `<element>` variants, but without lifetimes.
        
        #![no_std]
        #[cfg(feature = \"alloc\")]
        extern crate alloc;",
    )
    .into_bytes();
    for e in document.select(&selector) {
        let url = format!(
            "https://developer.mozilla.org{}",
            e.value().attr("href").unwrap()
        );
        // the name without the brackets
        let name = e.text().next().unwrap();
        let name = &name[1..name.len() - 1];
        let name = name.to_upper_camel_case();

        let resp = agent.get(&url).call().unwrap();
        let html = resp.into_string().unwrap();
        let document = Html::parse_document(&html);

        let deprecated = document
            .select(
                &Selector::parse(".main-page-content > .section-content > .notecard.deprecated")
                    .unwrap(),
            )
            .count()
            != 0;

        elems.push((name.clone(), deprecated));

        let mut attrs = global_attrs.clone();
        attrs.extend(get_attrs(&document, false));

        write_elem(
            get_mdn_doc(&document, &url),
            name.clone(),
            &attrs,
            deprecated,
            false,
            &mut buf,
        );

        let mut attrs = owned_global_attrs.clone();
        attrs.extend(get_attrs(&document, true));

        write_elem(
            get_mdn_doc(&document, &url),
            name,
            &attrs,
            deprecated,
            true,
            &mut buf,
        );
    }
    {
        let doc = "/// An unknown element.".to_string();
        let tag_name_doc = "The tag name of the element.".to_string();
        write_elem(
            doc.clone(),
            "Unknown".to_string(),
            &{
                let mut attrs = global_attrs.clone();
                attrs.insert(
                    "tag_name".to_string(),
                    (tag_name_doc.clone(), "&'life str".to_string(), false),
                );
                attrs
            },
            false,
            false,
            &mut buf,
        );
        write_elem(
            doc,
            "Unknown".to_string(),
            &{
                let mut attrs = owned_global_attrs.clone();
                attrs.insert(
                    "tag_name".to_string(),
                    (
                        tag_name_doc.clone(),
                        "alloc::string::String".to_string(),
                        false,
                    ),
                );
                attrs
            },
            false,
            true,
            &mut buf,
        );
        elems.push(("Unknown".to_string(), false));
    }

    write_elem_enum(&elems, &global_attrs, false, &mut buf);
    write_elem_enum(&elems, &owned_global_attrs, true, &mut buf);

    std::fs::write("src/lib.rs", buf).unwrap();

    std::process::Command::new("rustfmt")
        .arg("src/lib.rs")
        .status()
        .unwrap();
}

fn get_global_attrs(owned: bool) -> Vec<(String, (String, String, bool))> {
    let agent = ureq::agent();

    let resp = agent
        .get("https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes")
        .call()
        .unwrap();
    let html = resp.into_string().unwrap();
    let document = Html::parse_document(&html);
    let selector = Selector::parse("dl").unwrap();
    let dl = document.select(&selector).next().unwrap();
    let mut attrs = dl_to_attrs(dl, owned);
    attrs.push((
        "extra".to_string(),
        (
            "/// Extra attributes of the element. This is a map of attribute names to their values, and the attribute names are in lowercase."
                .to_string(),
            if owned {
                "alloc::collections::BTreeMap<alloc::string::String, alloc::string::String>"
            } else {
                "alloc::collections::BTreeMap<&'life str, &'life str>"
            }
            .to_string(),
            true,
        ),
    ));
    attrs
}

fn get_attrs(document: &Html, owned: bool) -> Vec<(String, (String, String, bool))> {
    let selector = Selector::parse(".section-content > dl").unwrap();

    if let Some(dl) = document.select(&selector).next() {
        dl_to_attrs(dl, owned)
    } else {
        Vec::new()
    }
}

// fn get_aria_attrs() -> Vec<(String, String)> {
//     let agent = ureq::agent();
//     // scrape MDN for all the elements
//     let resp = agent
//         .get("https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes")
//         .call()
//         .unwrap();
//     let html = resp.into_string().unwrap();
//     let document = Html::parse_document(&html);
//     let selector = Selector::parse(
//         "td:first-child > a[href^='/en-US/docs/Web/Accessibility/ARIA/Attributes/']:only-child",
//     )
//     .unwrap();
// }

fn get_mdn_doc(document: &Html, url: &str) -> String {
    let mut summary = document
        .select(&Selector::parse(".main-page-content > .section-content > p").unwrap())
        .map(|e| e.inner_html())
        .collect::<Vec<_>>();
    if summary.len() == 0 {
        summary = document
            .select(
                &Selector::parse(
                    ".main-page-content > section[aria-labelledby='summary'] > .section-content",
                )
                .unwrap(),
            )
            .map(|e| e.inner_html())
            .collect::<Vec<_>>();
    }
    let summary = summary
        .join("\n\n")
        .replace("<br>", "\n\n")
        .replace('\n', "\n/// ");
    format!("/// {}\n///\n/// More information: <{url}>", summary)
}

fn dl_to_attrs(dl: ElementRef, owned: bool) -> Vec<(String, (String, String, bool))> {
    let mut attrs = Vec::new();
    for e in dl
        .children()
        .filter_map(ElementRef::wrap)
        .filter(|e| e.value().name() == "dt")
    {
        let name = e.text().next().unwrap();
        let desc = e
            .next_sibling_element()
            .unwrap()
            .inner_html()
            .replace("<br>", "\n\n")
            .replace('\n', "\n/// ");
        let name = name.to_snake_case();

        let (ty, alloc) = match name.as_str() {
            "data" => (
                if owned {
                    "alloc::collections::BTreeMap<alloc::string::String, alloc::string::String>"
                } else {
                    "alloc::collections::BTreeMap<&'life str, &'life str>"
                },
                true,
            ),

            _ => (
                match name.as_str() {
                    "autofocus" | "checked" | "disabled" | "multiple" | "readonly" | "required"
                    | "selected" | "novalidate" | "formnovalidate" | "hidden" => "bool",
                    _ => {
                        if owned {
                            "alloc::string::String"
                        } else {
                            "&'life str"
                        }
                    }
                },
                false,
            ),
        };

        attrs.push((
            if ["type", "loop", "async", "for", "as"].contains(&&*name) {
                format!("{name}_")
            } else {
                name
            },
            (desc, ty.to_string(), alloc),
        ));
    }
    attrs
}

fn write_elem(
    doc: String,
    name: String,
    attrs: &BTreeMap<String, (String, String, bool)>,
    deprecated: bool,
    owned: bool,
    buf: &mut Vec<u8>,
) {
    writeln!(
        buf,
        "{0}
        {1}
        {2}
        #[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
        pub struct {name}{4}{3} {{
            {5}
        }}  
        #[allow(deprecated)]
        {2}
        impl{3} {name}{4}{3} {{
            /// Get the tag name of the element.
            /// This is the same as the name of the struct, in kebab-case.
            pub fn tag() -> &'static str {{
                \"{kebab}\"
            }}
        }}",
        doc,
        if deprecated { "#[deprecated]" } else { "" },
        if owned {
            "#[cfg(feature = \"alloc\")]"
        } else {
            ""
        },
        if owned { "" } else { "<'life>" },
        if owned { "Owned" } else { "" },
        attrs
            .iter()
            .format_with(",\n/// ", |(name, (desc, ty, alloc)), f| f(&format_args!(
                "{desc}
                {}
                pub {name}: core::option::Option<{ty}>",
                if *alloc {
                    "#[cfg(feature = \"alloc\")]"
                } else {
                    ""
                },
            ))),
        kebab = AsKebabCase(name.clone()),
    )
    .unwrap();
}

fn write_elem_enum(
    elems: &Vec<(String, bool)>,
    global_attrs: &BTreeMap<String, (String, String, bool)>,
    owned: bool,
    buf: &mut Vec<u8>,
) {
    writeln!(
        buf,
        "#[allow(deprecated)]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
        {}
        pub enum Element{} {{
            {}
        }}",
        if owned {
            "#[cfg(feature = \"alloc\")]"
        } else {
            ""
        },
        if owned { "Owned" } else { "<'life>" },
        elems
            .iter()
            .format_with(",\n", |(e, dep), f| f(&format_args!(
                "{} {e}({e}{})",
                if *dep { "#[deprecated]" } else { "" },
                if owned { "Owned" } else { "<'life>" },
            )))
    )
    .unwrap();
    writeln!(
        buf,
        "#[allow(deprecated)]
        {0}
        impl{1} Element{2}{1} {{
            /// Gets an element from a lowercase tag name.
            pub fn from_tag(tag: &str) -> Self {{
                match tag {{
                    {3},
                    _ => Self::default(),
                }}
            }}
            /// Gets the tag name of the element.
            pub fn tag(&self) -> &'static str {{
                match self {{
                    {4},
                }}
            }}
        }}",
        if owned {
            "#[cfg(feature = \"alloc\")]"
        } else {
            ""
        },
        if owned { "" } else { "<'life>" },
        if owned { "Owned" } else { "" },
        elems.iter().format_with(",\n", |(e, _), f| f(&format_args!(
            "\"{}\" => Self::{e}({e}{}::default())",
            AsKebabCase(e),
            if owned { "Owned" } else { "" },
        ))),
        elems.iter().format_with(",\n", |(e, _), f| f(&format_args!(
            "Self::{e}(_) => {e}::tag()",
        ))),
    )
    .unwrap();
    writeln!(
        buf,
        "#[allow(deprecated)]
        {0}
        impl{1} Element{2}{1} {{
            {3}
            {4}
        }}",
        if owned {
            "#[cfg(feature = \"alloc\")]"
        } else {
            ""
        },
        if owned { "" } else { "<'life>" },
        if owned { "Owned" } else { "" },
        global_attrs
            .iter()
            .format_with("\n", |(name, (desc, ty, alloc)), f| f(&format_args!(
                "{desc}
                {}
                pub fn {name}(&self) -> core::option::Option<{}{ty}> {{
                    match self {{
                        {}
                    }}
                }}",
                if *alloc || owned {
                    "#[cfg(feature = \"alloc\")]"
                } else {
                    ""
                },
                if *alloc || owned { "&" } else { "" },
                elems.iter().format_with(",", |(e, _), f| f(&format_args!(
                    "Self::{e}(e) => e.{name}{}",
                    if *alloc || owned { ".as_ref()" } else { "" }
                )))
            ))),
        global_attrs
            .iter()
            .format_with("\n", |(name, (desc, ty, alloc)), f| f(&format_args!(
                "{desc}
                    {}
                    pub fn set_{name}(&mut self, val: {ty}) {{
                        match self {{
                            {}
                        }};
                    }}",
                if *alloc || owned {
                    "#[cfg(feature = \"alloc\")]"
                } else {
                    ""
                },
                elems.iter().format_with(",", |(e, _), f| f(&format_args!(
                    "Self::{e}(e) => e.{name}.replace(val)",
                )))
            )))
    )
    .unwrap();
    writeln!(
        buf,
        "#[allow(deprecated)]
        {0}
        impl{1} Default for Element{2}{1} {{
            fn default() -> Self {{
                Self::Unknown(Unknown{2}::default())
            }}
        }}",
        if owned {
            "#[cfg(feature = \"alloc\")]"
        } else {
            ""
        },
        if owned { "" } else { "<'life>" },
        if owned { "Owned" } else { "" },
    )
    .unwrap();
}
