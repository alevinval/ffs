use crate::{Error, filesystem::MAX_FILENAME_LEN};

pub fn validate(path: &str) -> Result<(), Error> {
    let first = first_component(path);
    if first == path && path.len() < MAX_FILENAME_LEN {
        return Ok(());
    }
    if first.len() >= MAX_FILENAME_LEN {
        return Err(Error::FileNameTooLong);
    }
    validate(tail(path))
}

pub fn dirname(path: &str) -> &str {
    let path = norm(path);
    path.rsplit_once('/').map(|(dirname, _)| dirname).unwrap_or_default()
}

pub fn basename(path: &str) -> &str {
    let path = norm(path);
    path.rsplit_once('/').map(|(_, basename)| basename).unwrap_or(path)
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
    let path = norm(path);
    path.split('/').next().unwrap_or("")
}

pub fn norm(file_name: &str) -> &str {
    file_name.trim_start_matches('/').trim_end_matches('/')
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn basename_and_dirname() {
        let name = "/path/to/file.txt";
        assert_eq!("path/to", dirname(name));
        assert_eq!("file.txt", basename(name));

        let name = "file.txt";
        assert_eq!("", dirname(name));
        assert_eq!("file.txt", basename(name));

        let name = "/";
        assert_eq!("", dirname(name));
        assert_eq!("", basename(name));

        let name = "";
        assert_eq!("", dirname(name));
        assert_eq!("", basename(name));
    }

    #[test]
    fn tail_path() {
        let actual = tail("foo/bar/baz");
        assert_eq!("bar/baz", actual);
        assert_eq!("baz", tail(actual));
        assert_eq!("baz", tail("baz"))
    }
}
