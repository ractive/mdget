pub fn greeting() -> &'static str {
    "Hello from mdget!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greeting() {
        assert_eq!(greeting(), "Hello from mdget!");
    }
}
