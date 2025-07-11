use crate::{Error, filesystem::Name};

pub const SEPARATOR: char = '/';

pub fn validate(path: &str) -> Result<(), Error> {
    let first_component = first_component(path);
    if first_component == path && path.len() < Name::LEN {
        return Ok(());
    }
    if first_component.len() >= Name::LEN {
        return Err(Error::FileNameTooLong);
    }
    validate(tail(path))
}

pub fn dirname(path: &str) -> &str {
    norm(path).rsplit_once(SEPARATOR).map(|(dirname, _)| dirname).unwrap_or_default()
}

pub fn tail(path: &str) -> &str {
    let path = norm(path);
    if dirname(path).is_empty() {
        return path;
    }
    let first = first_component(path);
    norm(path.strip_prefix(first).unwrap())
}

pub fn first_component(path: &str) -> &str {
    norm(path).split(SEPARATOR).next().unwrap_or("")
}

fn norm(path: &str) -> &str {
    path.trim_start_matches(SEPARATOR).trim_end_matches(SEPARATOR)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_validate() {
        let mut input = "a".repeat(Name::LEN - 1);
        assert!(validate(&input).is_ok());

        input += "/a/b/c/d/";
        assert!(validate(&input).is_ok());

        input += "a".repeat(Name::LEN).as_str();
        assert_eq!(Error::FileNameTooLong, validate(&input).unwrap_err());
    }

    #[test]
    fn test_dirname() {
        assert_eq!("", dirname(""));

        assert_eq!("", dirname("/"));

        let input = "/path/to/file.txt";
        assert_eq!("path/to", dirname(input));

        let input = "/path/to/file.txt/";
        assert_eq!("path/to", dirname(input));

        let input = "file.txt";
        assert_eq!("", dirname(input));
    }

    #[test]
    fn test_tail() {
        let input = "foo/bar/baz";
        assert_eq!("bar/baz", tail(input));
        assert_eq!("baz", tail(tail(input)));
        assert_eq!("baz", tail(tail(tail(input))));
    }
}
