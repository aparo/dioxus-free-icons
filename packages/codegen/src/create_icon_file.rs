use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use heck::ToSnakeCase;
use heck::ToUpperCamelCase;
use regex::Regex;
use scraper::node::Element;
use scraper::ElementRef;
use scraper::Html;
use walkdir::WalkDir;

const ICON_TEMPLATE: &str = r#"pub struct {ICON_NAME};
impl IconShape for {ICON_NAME} {
    fn view_box(&self) -> String {
        String::from("{VIEW_BOX}")
    }
    fn xmlns(&self) -> String {
        String::from("{XMLNS}")
    }
    fn child_elements(&self) -> LazyNodes {
        rsx! {
{CHILD_ELEMENTS}
        }
    }
}
"#;

pub fn create_icon_file(svg_path: &str, output_path: &str, icon_prefix: &str) {
    let dir_entries = WalkDir::new(svg_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();

    let files = dir_entries
        .into_iter()
        .filter(|e| e.path().extension() == Some(OsStr::new("svg")))
        .map(|dir| PathBuf::from(dir.path()))
        .collect::<Vec<_>>();

    let icon_file = files
        .into_iter()
        .map(|file| {
            let svg_str = fs::read_to_string(&file).unwrap();
            let fragment = Html::parse_fragment(&svg_str);

            let elements = fragment
                .tree
                .nodes()
                .filter_map(|node| {
                    if node.value().is_element() {
                        let element = ElementRef::wrap(node).unwrap().value();
                        if element.attrs.len() != 0 {
                            return Some(element);
                        }
                    }
                    return None;
                })
                .collect::<Vec<_>>();

            let svg_element = &elements[0];
            let svg_child_elements = &elements[1..];
            let icon_name = icon_name(&file);
            let (view_box, xmlns) = extract_svg_attrs(svg_element);
            let child_elements = extract_svg_child_elements(svg_child_elements);

            ICON_TEMPLATE
                .replace("{ICON_NAME}", &format!("{}{}", icon_prefix, &icon_name))
                .replace("{VIEW_BOX}", &view_box)
                .replace("{XMLNS}", &xmlns)
                .replace("{CHILD_ELEMENTS}", &child_elements)
        })
        .collect::<Vec<_>>()
        .join("\n");

    // write to file
    let mut file = File::create(output_path).unwrap();
    file.write_all(
        format!(
            "{}\n\n{}",
            "use super::super::IconShape;\nuse dioxus::prelude::*;", icon_file
        )
        .as_bytes(),
    )
    .unwrap();
    file.flush().unwrap();
}

fn icon_name(path: &PathBuf) -> String {
    let filename = path.file_name().unwrap().to_str().unwrap();
    let name = filename.split('.').next().unwrap();
    name.to_upper_camel_case()
}

pub fn extract_svg_attrs(element: &Element) -> (String, String) {
    let view_box = element.attr("viewBox").unwrap();
    let xmlns = element
        .attr("xmlns")
        .unwrap_or("http://www.w3.org/2000/svg");
    return (String::from(view_box), String::from(xmlns));
}

pub fn extract_svg_child_elements(elements: &[&Element]) -> String {
    elements
        .into_iter()
        .map(|element| {
            let tag_name = element.name();
            let mut element_attrs = element
                .attrs()
                .filter_map(|(name, value)| {
                    let re = Regex::new(r"^data-.*$").unwrap();
                    if !re.is_match(&name) {
                        Some(format!(
                            "                {}: \"{}\",",
                            name.to_snake_case(),
                            value
                        ))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            element_attrs.sort();
            let attrs_str = element_attrs.join("\n");
            "            {TAG_NAME} {\n{ATTRS}\n            }"
                .replace("{TAG_NAME}", tag_name)
                .replace("{ATTRS}", &attrs_str)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
