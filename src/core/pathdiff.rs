use std::path::*;

/// Construct a relative path from a provided base directory path to the provided path.
pub fn diff_paths<P, B>(path: P, base: B) -> Option<PathBuf>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    let path = path.as_ref();
    let base = base.as_ref();

    if path.is_absolute() != base.is_absolute() {
        if path.is_absolute() {
            Some(PathBuf::from(path))
        } else {
            None
        }
    } else {
        let mut ita = path.components();
        let mut itb = base.components();
        let mut comps: Vec<Component> = vec![];
        loop {
            match (ita.next(), itb.next()) {
                (None, None) => break,
                (Some(a), None) => {
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
                (None, _) => comps.push(Component::ParentDir),
                (Some(a), Some(b)) if comps.is_empty() && a == b => (),
                (Some(a), Some(Component::CurDir)) => comps.push(a),
                (Some(_), Some(Component::ParentDir)) => return None,
                (Some(a), Some(_)) => {
                    comps.push(Component::ParentDir);
                    for _ in itb {
                        comps.push(Component::ParentDir);
                    }
                    comps.push(a);
                    comps.extend(ita.by_ref());
                    break;
                }
            }
        }
        Some(comps.iter().map(|c| c.as_os_str()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_absolute() {
        fn abs(path: &str) -> String {
            if cfg!(windows) {
                format!("C:\\{}", path)
            } else {
                format!("/{}", path)
            }
        }

        assert_eq!(diff_paths(abs("foo"), abs("bar")), Some("../foo".into()));
        assert_eq!(diff_paths(abs("foo"), "bar"), Some(abs("foo").into()));
        assert_eq!(diff_paths("foo", abs("bar")), None);
        assert_eq!(diff_paths("foo", "bar"), Some("../foo".into()));
    }

    #[test]
    fn test_identity() {
        assert_eq!(diff_paths(".", "."), Some("".into()));
        assert_eq!(diff_paths("../foo", "../foo"), Some("".into()));
        assert_eq!(diff_paths("./foo", "./foo"), Some("".into()));
        assert_eq!(diff_paths("/foo", "/foo"), Some("".into()));
        assert_eq!(diff_paths("foo", "foo"), Some("".into()));
    }

    #[test]
    fn test_subset() {
        assert_eq!(diff_paths("foo", "fo"), Some("../foo".into()));
        assert_eq!(diff_paths("fo", "foo"), Some("../fo".into()));
    }

    #[test]
    fn test_empty() {
        assert_eq!(diff_paths("", ""), Some("".into()));
        assert_eq!(diff_paths("foo", ""), Some("foo".into()));
        assert_eq!(diff_paths("", "foo"), Some("..".into()));
    }

    #[test]
    fn test_relative() {
        assert_eq!(diff_paths("../foo", "../bar"), Some("../foo".into()));
        assert_eq!(diff_paths("../foo", "../foo/bar/baz"), Some("../..".into()));
        assert_eq!(
            diff_paths("../foo/bar/baz", "../foo"),
            Some("bar/baz".into())
        );

        assert_eq!(diff_paths("foo/bar/baz", "foo"), Some("bar/baz".into()));
        assert_eq!(diff_paths("foo/bar/baz", "foo/bar"), Some("baz".into()));
        assert_eq!(diff_paths("foo/bar/baz", "foo/bar/baz"), Some("".into()));
        assert_eq!(diff_paths("foo/bar/baz", "foo/bar/baz/"), Some("".into()));
    }

    #[test]
    fn test_current_directory() {
        assert_eq!(diff_paths(".", "foo"), Some("../.".into()));
        assert_eq!(diff_paths("foo", "."), Some("foo".into()));
        assert_eq!(diff_paths("/foo", "/."), Some("foo".into()));
    }
}
