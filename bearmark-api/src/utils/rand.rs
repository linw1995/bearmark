pub fn rand_str(length: usize) -> String {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rand_str() {
        let s1 = rand_str(10);
        assert_eq!(s1.len(), 10);
        let s2 = rand_str(10);
        assert_eq!(s2.len(), 10);
        assert_ne!(s1, s2)
    }
}
