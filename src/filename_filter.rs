use regex::bytes::Regex;

#[derive(Debug, PartialEq)]
pub enum FileNameMatch {
    None,
    SingleRange((usize, usize)),
}

pub trait FileNameFilter {
    fn filter(&self, filename: &[u8]) -> Option<FileNameMatch>;
}

pub struct SubstringFilter {
    pattern: Vec<u8>,
}

impl SubstringFilter {
    pub fn new(pattern: &str) -> Self {
        SubstringFilter {
            pattern: pattern.as_bytes().to_vec(),
        }
    }
}

impl FileNameFilter for SubstringFilter {
    fn filter(&self, filename: &[u8]) -> Option<FileNameMatch> {
        filename
            .windows(self.pattern.len())
            .position(|window| window == self.pattern.as_slice())
            .map(|start| FileNameMatch::SingleRange((start, start + self.pattern.len())))
    }
}

#[derive(Debug)]
pub struct RegexFilter {
    regex: Regex,
}

impl RegexFilter {
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        match Regex::new(pattern) {
            Ok(regex) => Ok(RegexFilter { regex }),
            Err(err) => Err(err),
        }
    }
}

impl FileNameFilter for RegexFilter {
    fn filter(&self, filename: &[u8]) -> Option<FileNameMatch> {
        self.regex
            .find(filename)
            .map(|m| FileNameMatch::SingleRange((m.start(), m.end())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substring_filter_returns_none_when_no_match() {
        let filter = SubstringFilter::new("abc");
        let result = filter.filter(b"def");
        assert_eq!(result, None);
    }

    #[test]
    fn substring_filter_returns_match_range_when_pattern_found() {
        let filter = SubstringFilter::new("abc");
        /* cspell:disable-next-line */
        let result = filter.filter(b"xyzabc123");
        assert_eq!(result, Some(FileNameMatch::SingleRange((3, 6))));
    }

    #[test]
    fn substring_filter_returns_first_match_range_when_multiple_patterns_found() {
        let filter = SubstringFilter::new("abc");
        /* cspell:disable-next-line */
        let result = filter.filter(b"xyzabc123abc");
        assert_eq!(result, Some(FileNameMatch::SingleRange((3, 6))));
    }

    #[test]
    fn regex_filter_returns_none_when_no_match() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter(b"abc");
        assert_eq!(result, None);
    }

    #[test]
    fn regex_filter_returns_match_range_when_pattern_found() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter(b"abc123def");
        assert_eq!(result, Some(FileNameMatch::SingleRange((3, 6))));
    }

    #[test]
    fn regex_filter_returns_first_match_range_when_multiple_patterns_found() {
        let filter = RegexFilter::new(r"\d+").unwrap();
        let result = filter.filter(b"abc123def456");
        assert_eq!(result, Some(FileNameMatch::SingleRange((3, 6))));
    }

    #[test]
    fn regex_filter_returns_error_when_invalid_pattern() {
        let filter = RegexFilter::new(r"(").unwrap_err();
        assert_eq!(filter.to_string().contains("regex parse error"), true);
    }
}
