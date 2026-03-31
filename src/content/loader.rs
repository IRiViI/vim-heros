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

/// Load intro segment for a specific level (e.g., "1-1").
/// Intro segments live in content/{language}/intro/ and have `intro = true`.
pub fn load_intro_segment(language: &str, level_id: &str) -> Option<Segment> {
    // Look in the intro zone directory
    let path = format!("{}/intro", language);
    let dir = CONTENT_DIR.get_dir(&path)?;

    for file in dir.files() {
        let ext = file.path().extension()?;
        if ext != "toml" {
            continue;
        }
        let contents = file.contents_utf8()?;
        if let Ok(seg) = Segment::from_toml(contents) {
            if seg.meta.intro {
                if let Some(ref target) = seg.meta.intro_level {
                    if target == level_id {
                        return Some(seg);
                    }
                }
            }
        }
    }

    None
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
    fn test_load_python_junior() {
        let segments = load_segments("python", "junior");
        assert!(
            segments.len() >= 30,
            "Expected at least 30 Python junior segments, got {}",
            segments.len()
        );
        for seg in &segments {
            assert_eq!(seg.meta.language, "python");
            assert_eq!(seg.meta.zone, "junior");
            assert!(!seg.code.content.is_empty());
            assert!(!seg.tasks.is_empty(), "Segment {} has no tasks", seg.meta.id);
        }
    }

    #[test]
    fn test_load_typescript_starter() {
        let segments = load_segments("typescript", "starter");
        assert!(
            segments.len() >= 30,
            "Expected at least 30 TypeScript starter segments, got {}",
            segments.len()
        );
        for seg in &segments {
            assert_eq!(seg.meta.language, "typescript");
            assert_eq!(seg.meta.zone, "starter");
            assert!(!seg.code.content.is_empty());
        }
    }

    #[test]
    fn test_load_typescript_junior() {
        let segments = load_segments("typescript", "junior");
        assert!(
            segments.len() >= 30,
            "Expected at least 30 TypeScript junior segments, got {}",
            segments.len()
        );
        for seg in &segments {
            assert_eq!(seg.meta.language, "typescript");
            assert_eq!(seg.meta.zone, "junior");
            assert!(!seg.code.content.is_empty());
            assert!(!seg.tasks.is_empty(), "Segment {} has no tasks", seg.meta.id);
        }
    }

    #[test]
    fn test_load_python_nav() {
        let segments = load_segments("python", "nav");
        assert!(
            segments.len() >= 6,
            "Expected at least 6 Python nav segments, got {}",
            segments.len()
        );
        for seg in &segments {
            assert_eq!(seg.meta.language, "python");
            assert_eq!(seg.meta.zone, "nav");
            assert!(!seg.code.content.is_empty());
            assert!(!seg.tasks.is_empty(), "Segment {} has no tasks", seg.meta.id);
        }
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let segments = load_segments("haskell", "starter");
        assert!(segments.is_empty());
    }
}
