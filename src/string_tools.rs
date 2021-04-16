pub fn ensure_camel_case(str: &mut String) {
    if let Some(c) = str.get_mut(0..1) {
        c.make_ascii_lowercase();
    }
}
