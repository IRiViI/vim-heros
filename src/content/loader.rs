use include_dir::{include_dir, Dir};

use super::segment::Segment;

/// All content segments embedded in the binary at compile time.
static CONTENT_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/content");

/// Load all segments for a given language and zone.
pub fn load_segments(language: &str, zone: &str) -> Vec<Segment> {
    let mut segments = Vec::new();

    // Path: content/{language}/{zone}/
    let path = format!("{}/{}", language, zone);
    let Some(dir) = CONTENT_DIR.get_dir(&path) else {
        return segments;
    };

    for file in dir.files() {
        let Some(ext) = file.path().extension() else {
            continue;
        };
        if ext != "toml" {
            continue;
        }
        let Some(contents) = file.contents_utf8() else {
            continue;
        };
        match Segment::from_toml(contents) {
            Ok(seg) => segments.push(seg),
            Err(e) => {
                eprintln!(
                    "Warning: failed to parse {:?}: {}",
                    file.path(),
                    e
                );
            }
        }
    }

    segments
}

/// Load all segments across all zones for a language.
pub fn load_all_segments(language: &str) -> Vec<Segment> {
    let mut all = Vec::new();
    for zone in &["starter", "junior", "medior", "senior"] {
        all.extend(load_segments(language, zone));
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_python_starter() {
        let segments = load_segments("python", "starter");
        assert!(
            segments.len() >= 5,
            "Expected at least 5 Python starter segments, got {}",
            segments.len()
        );
        for seg in &segments {
            assert_eq!(seg.meta.language, "python");
            assert_eq!(seg.meta.zone, "starter");
            assert!(!seg.code.content.is_empty());
        }
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let segments = load_segments("haskell", "starter");
        assert!(segments.is_empty());
    }
}
