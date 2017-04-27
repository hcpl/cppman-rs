use std::collections::HashMap;
use std::collections::hash_map::Entry::Occupied;

use select::document::Document;
use select::node::{Node, Data};
use select::predicate::{Predicate, Class};

use ::formatter::utils::{HtmlError, repeat_char, get_attrs};


fn traverse(node: &Node, depth: usize) {
    println!("{}{:?}: {:?} {}",
        repeat_char(' ', 2),
        node.name(),
        get_attrs(node),
        node.text());

    for c in node.children() {
        traverse(&c, depth + 2);
    }
}

fn get_row_width(node: &Node) -> Result<usize, HtmlError> {
    let mut total = 0;
    assert_eq!(try!(node.name().ok_or(HtmlError::NotElement)), "tr");

    for c in node.children() {
        if let Some(value) = c.attr("colspan") {
            total += try!(value.parse::<usize>().map_err(HtmlError::from_error));
        } else {
            total += 1;
        }
    }

    Ok(total)
}

fn scan_format<'a>(node: &Node,
                   index: Option<usize>,
                   width: Option<usize>,
                   rowspan: &'a mut HashMap<usize, usize>) -> Result<String, HtmlError> {
    let index = index.unwrap_or(0);
    let mut width = width.unwrap_or(0);

    let mut format_str = String::new();
    let expand_char = "";

    let name = try!(node.name().ok_or(HtmlError::NotElement));
    let attrs = try!(get_attrs(node));

    if name == "th" || name == "td" {
        let extend = (width == 3 && index == 1) ||
                     (width != 3 && width < 5 && index == width - 1);

        if name == "th" {
            format_str.push_str(&format!("c{} ", if extend { expand_char } else { "" }));
        } else {
            format_str.push_str(&format!("l{} ", if extend { expand_char } else { "" }));
        }

        if let Some(cspan) = attrs.get("colspan") {
            for _ in 0..(try!(cspan.parse::<usize>().map_err(HtmlError::from_error)) - 1) {
                format_str.push_str("s ");
            }
        }

        if let Some(rspan) = attrs.get("rowspan") {
            let rspan = try!(rspan.parse::<usize>().map_err(HtmlError::from_error));
            if rspan > 1 {
                rowspan.insert(index, rspan);
            }
        }
    }

    if name == "tr" && rowspan.len() > 0 {
        let mut ci = 0;

        for i in 0..width {
            if rowspan.contains_key(&i) {
                format_str.push_str("^ ");
                if rowspan[&i] == 1 {
                    rowspan.remove(&i);
                } else {
                    *rowspan.get_mut(&i).unwrap() -= 1;
                }
            } else {
                // There is a row span, but the current number of column is
                // not enough. Pad empty node when this happens.
                if ci >= node.children().count() {
                    // TODO: append new children
                    unimplemented!();
                }

                format_str.push_str(&try!(scan_format(
                    &node.children().nth(ci).unwrap(),
                    Some(i), Some(width), rowspan)));
                ci += 1;
            }
        }
    } else {
        if let Some(first_child) = node.children().nth(0) {
            if try!(first_child.name().ok_or(HtmlError::NotElement)) == "tr" {
                width = try!(get_row_width(&first_child));
            }
        }

        for (i, c) in node.children().enumerate() {
            format_str.push_str(&try!(scan_format(&c, Some(i), Some(width), rowspan)))
        }
    }

    if name == "table" {
        format_str.push_str(".\n");
    } else if name == "tr" {
        format_str.push_str("\n");
    }

    Ok(format_str)
}

fn gen<'a>(node: &Node,
           output: &mut String,
           index: Option<usize>,
           last: Option<bool>,
           rowspan: &'a mut HashMap<usize, usize>) -> Result<(), HtmlError> {
    let index = index.unwrap_or(0);
    let last = last.unwrap_or(false);

    let name = try!(node.name().ok_or(HtmlError::NotElement));
    let attrs = try!(get_attrs(node));

    if name == "table" {
        let mut scan_format_rowspan = HashMap::new();

        output.push_str(".TS\n");
        output.push_str("allbox tab(|);\n");
        output.push_str(&try!(scan_format(node, None, None, &mut scan_format_rowspan)));
    } else if name == "th" || name == "td" {
        output.push_str(&format!("T{{\n{}", node.text()));

        if let Some(rspan) = attrs.get("rowspan") {
            let rspan = try!(rspan.parse::<usize>().map_err(HtmlError::from_error));
            if rspan > 1 {
                rowspan.insert(index, rspan - 1);
            }
        }
    } else {
        output.push_str(&node.text());
    }

    if name == "tr" && rowspan.len() > 0 {
        let total = rowspan.len() + node.children().count();
        let mut ci = 0;

        for i in 0..total {
            /*if let Some(rspan) = rowspan.get(&i) {
                output.push_str(&format!("\\^{}", if i < total - 1 { "|" } else { "" }));
                if *rspan == 1 {
                    rowspan.remove(&i);
                } else {
                    *rowspan.get_mut(&i).unwrap() -= 1;
                }
            } else {
                if ci >= node.children().count() {
                    // TODO: TODO!!!!
                    unimplemented!();
                }

                gen(&node.children().nth(ci).unwrap(),
                    output, Some(i), Some(i == total - 1), rowspan);
                ci += 1;
            }*/

            let mut has_entry = false;

            if let Occupied(mut rspan) = rowspan.entry(i) {
                output.push_str(&format!("\\^{}", if i < total - 1 { "|" } else { "" }));
                if *rspan.get() == 1 {
                    rspan.remove_entry();
                } else {
                    *rspan.get_mut() -= 1;
                }

                has_entry = true;
            }

            if !has_entry {
                if ci >= node.children().count() {
                    // TODO: TODO!!!!
                    unimplemented!();
                }

                gen(&node.children().nth(ci).unwrap(),
                    output, Some(i), Some(i == total - 1), rowspan);
                ci += 1;
            }
        }
    } else {
        for (i, c) in node.children().enumerate() {
            match *c.data() {
                Data::Element(_, _) => try!(gen(&c, output, Some(i), Some(i == node.children().count() - 1), rowspan)),
                _ => (),
            }
        }
    }

    if name == "table" {
        output.push_str(".TE\n");
        output.push_str(".sp\n.sp\n");
    } else if name == "tr" {
        output.push_str("\n");
    } else if name == "th" || name == "td" {
        output.push_str(&format!("\nT}}{}", if !last { "|" } else { "" }))
    }

    Ok(())
}

pub fn parse_table(html: &str) -> String {
    let doc = Document::from(html);
    let root = doc.nth(0).unwrap();
    let mut output = String::new();
    let mut gen_rowspan = HashMap::new();

    gen(&root, &mut output, None, None, &mut gen_rowspan);
    output
}
