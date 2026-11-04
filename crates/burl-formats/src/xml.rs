//! The TinyXML behaviours the SVN and cvs2cl parsers depend on, on top of `roxmltree`:
//! `GetText()` returns the value of the FIRST child only if it is a text node; TinyXML
//! condenses whitespace in text (leading/trailing dropped, runs collapsed to one space) and
//! never creates whitespace-only text nodes. Entities are decoded by both parsers.

use roxmltree::Node;

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// `TiXmlBase::ReadText` with `condenseWhiteSpace` (the default).
pub fn condense(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut whitespace = false;
    for ch in text.chars() {
        if ch.is_ascii() && is_c_space(ch as u8) {
            whitespace = true;
        } else {
            if whitespace && !out.is_empty() {
                out.push(' ');
            }
            whitespace = false;
            out.push(ch);
        }
    }
    out
}

/// `TiXmlElement::GetText()`; `None` where TinyXML returns NULL.
pub fn get_text(elem: Node) -> Option<String> {
    let mut first = elem.first_child();
    if let Some(n) = first {
        if n.is_text() {
            let t = condense(n.text().unwrap_or(""));
            if !t.is_empty() {
                return Some(t);
            }
            first = n.next_sibling();
        }
    }
    match first {
        Some(n) if n.is_text() => Some(condense(n.text().unwrap_or(""))).filter(|t| !t.is_empty()),
        _ => None,
    }
}

/// `FirstChildElement(name)`.
pub fn first_child_element<'a, 'input>(
    elem: Node<'a, 'input>,
    name: &str,
) -> Option<Node<'a, 'input>> {
    elem.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

/// `FirstChildElement(name)` then `NextSiblingElement()` (any name) iteration.
pub fn sibling_elements_from<'a, 'input>(
    elem: Node<'a, 'input>,
    name: &str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    let name = name.to_string();
    elem.children()
        .filter(|c| c.is_element())
        .skip_while(move |c| c.tag_name().name() != name)
}

/// TinyXML parses raw bytes; `roxmltree` needs a `str`. Invalid UTF-8 can only live in
/// text and attribute values (markup is ASCII), where `filter_utf8` is what the commit
/// constructor would do to it anyway.
pub fn parse_fragment(bytes: &[u8]) -> Option<roxmltree::Document<'_>> {
    let text = std::str::from_utf8(bytes).ok()?;
    roxmltree::Document::parse(text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condense_like_tinyxml() {
        assert_eq!(condense("  bob   smith \n"), "bob smith");
        assert_eq!(condense("\n   "), "");
        assert_eq!(condense("/blah"), "/blah");
    }

    #[test]
    fn get_text_first_child_only() {
        let doc =
            roxmltree::Document::parse("<r><a>x<b/>y</a><c><b/>z</c><d>  </d><e>&amp;&lt;</e></r>")
                .unwrap();
        let root = doc.root_element();
        let get = |n: &str| first_child_element(root, n).unwrap();
        assert_eq!(get_text(get("a")).as_deref(), Some("x"));
        assert_eq!(get_text(get("c")), None);
        assert_eq!(get_text(get("d")), None);
        assert_eq!(get_text(get("e")).as_deref(), Some("&<"));
    }

    /// G2: where Burl's text differs from TinyXML's. The C++ column was measured by compiling
    /// `src/tinyxml` and calling `GetText()` on the same input; the CDATA node is kept verbatim there.
    #[test]
    fn deviation_g2_cdata_and_entity_differences_from_tinyxml() {
        let get = |xml: &str| {
            let doc = parse_fragment(xml.as_bytes())?;
            Some(get_text(
                first_child_element(doc.root_element(), "path").unwrap(),
            ))
        };
        // (input, TinyXML GetText, Burl get_text)
        // same: plain text, entities, a comment before text, text before a comment
        assert_eq!(
            get("<r><path>&#65;&#x42;&apos;</path></r>"),
            Some(Some("AB'".into()))
        );
        assert_eq!(get("<r><path><!-- c -->z</path></r>"), Some(None));
        assert_eq!(
            get("<r><path>a<!-- c -->b</path></r>"),
            Some(Some("a".into()))
        );
        // differs: TinyXML "/a  b" (CDATA is not condensed), Burl condenses
        assert_eq!(
            get("<r><path><![CDATA[/a  b]]></path></r>"),
            Some(Some("/a b".into()))
        );
        // differs: TinyXML "x" (text and CDATA are separate nodes), Burl merges them
        assert_eq!(
            get("<r><path>x<![CDATA[y]]></path></r>"),
            Some(Some("xy".into()))
        );
        // differs: TinyXML "" (a non-NULL empty string), Burl None
        assert_eq!(get("<r><path><![CDATA[]]></path></r>"), Some(None));
        // differs: TinyXML is lenient ("bogus;"), roxmltree rejects the whole entry
        assert_eq!(get("<r><path>&bogus;</path></r>"), None);
    }
}
