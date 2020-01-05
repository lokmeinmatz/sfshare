#[inline]
pub fn s_contains(vec: &Vec<String>, s: &str) -> bool {
    vec.iter().any(|e| e == s)
}
