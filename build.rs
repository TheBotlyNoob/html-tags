use scraper::{Html, Selector};
use std::io::Write;

fn main() {
    let agent = ureq::agent();
    // scrape MDN for all the elements
    let resp = agent
        .get("https://developer.mozilla.org/en-US/docs/Web/HTML/Element")
        .call()
        .unwrap();
    let html = resp.into_string().unwrap();
    let document = Html::parse_document(&html);
    let selector =
        Selector::parse("td:first-child > a[href^='/en-US/docs/Web/HTML/Element/']:only-child")
            .unwrap();

    let mut buf = Vec::new();
    for e in document.select(&selector) {
        let url = e.value().attr("href").unwrap();
        // the name without the brackets
        let name = e.text().next().unwrap();
        let name = &name[1..name.len() - 1];

        let resp = agent
            .get(&format!("https://developer.mozilla.org{}", url))
            .call()
            .unwrap();
        let html = resp.into_string().unwrap();
        let document = Html::parse_document(&html);
        let summary = document
            .select(&Selector::parse(".main-page-content > .section-content > p").unwrap())
            .map(|e| e.inner_html())
            .collect::<Vec<_>>()
            .join("<br/>");
        let depreciated = document
            .select(
                &Selector::parse(".main-page-content > .section-content > .notecard.deprecated")
                    .unwrap(),
            )
            .count()
            != 0;

        writeln!(
            buf,
            "#[doc = \"{}\"] {} pub struct {};",
            summary.replace('"', "\\\""),
            if depreciated { "#[deprecated]" } else { "" },
            heck::AsUpperCamelCase(name)
        )
        .unwrap();
    }
    std::fs::write(format!("{}/gen.rs", std::env::var("OUT_DIR").unwrap()), buf).unwrap();
}
