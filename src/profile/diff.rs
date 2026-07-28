use super::crud;

#[derive(Debug, PartialEq)]
pub enum DiffKind {
    Added,
    Removed,
    Unchanged,
}

#[derive(Debug, PartialEq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub content: String,
}

pub fn diff(name: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let name = crud::resolve(name)?;
    let raw = crud::read_raw(&name)?;

    let processed = match crud::read_processed(&name) {
        Some(p) => p,
        None => {
            println!("no processed config available yet (run overwrite first)");
            return Ok(());
        }
    };

    let diff_lines = line_diff(&raw, &processed);
    render_diff(&diff_lines);
    Ok(())
}

pub fn line_diff(old: &str, new: &str) -> Vec<DiffLine> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut result = Vec::new();
    let mut old_idx = 0;
    let mut new_idx = 0;

    while old_idx < old_lines.len() || new_idx < new_lines.len() {
        if old_idx >= old_lines.len() {
            result.push(DiffLine {
                kind: DiffKind::Added,
                content: new_lines[new_idx].to_string(),
            });
            new_idx += 1;
        } else if new_idx >= new_lines.len() {
            result.push(DiffLine {
                kind: DiffKind::Removed,
                content: old_lines[old_idx].to_string(),
            });
            old_idx += 1;
        } else if old_lines[old_idx] == new_lines[new_idx] {
            result.push(DiffLine {
                kind: DiffKind::Unchanged,
                content: old_lines[old_idx].to_string(),
            });
            old_idx += 1;
            new_idx += 1;
        } else {
            // Simple heuristic: look ahead to find matching line
            let old_ahead = old_lines[old_idx + 1..]
                .iter()
                .position(|l| *l == new_lines[new_idx]);
            let new_ahead = new_lines[new_idx + 1..]
                .iter()
                .position(|l| *l == old_lines[old_idx]);

            match (old_ahead, new_ahead) {
                (Some(o), Some(n)) if o <= n => {
                    // Remove old lines until match
                    result.push(DiffLine {
                        kind: DiffKind::Removed,
                        content: old_lines[old_idx].to_string(),
                    });
                    old_idx += 1;
                }
                (Some(_), Some(_)) => {
                    // Add new lines until match
                    result.push(DiffLine {
                        kind: DiffKind::Added,
                        content: new_lines[new_idx].to_string(),
                    });
                    new_idx += 1;
                }
                (Some(_), None) => {
                    result.push(DiffLine {
                        kind: DiffKind::Removed,
                        content: old_lines[old_idx].to_string(),
                    });
                    old_idx += 1;
                }
                (None, Some(_)) => {
                    result.push(DiffLine {
                        kind: DiffKind::Added,
                        content: new_lines[new_idx].to_string(),
                    });
                    new_idx += 1;
                }
                (None, None) => {
                    result.push(DiffLine {
                        kind: DiffKind::Removed,
                        content: old_lines[old_idx].to_string(),
                    });
                    result.push(DiffLine {
                        kind: DiffKind::Added,
                        content: new_lines[new_idx].to_string(),
                    });
                    old_idx += 1;
                    new_idx += 1;
                }
            }
        }
    }

    result
}

pub fn render_diff(diff: &[DiffLine]) {
    for line in diff {
        match line.kind {
            DiffKind::Added => println!("\x1b[32m+ {}\x1b[0m", line.content),
            DiffKind::Removed => println!("\x1b[31m- {}\x1b[0m", line.content),
            DiffKind::Unchanged => println!("  {}", line.content),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_diff_identical() {
        let diff = line_diff("a\nb\nc", "a\nb\nc");
        assert_eq!(diff.len(), 3);
        assert!(diff.iter().all(|d| d.kind == DiffKind::Unchanged));
    }

    #[test]
    fn test_line_diff_completely_different() {
        let diff = line_diff("a\nb", "x\ny");
        assert!(diff.iter().all(|d| d.kind != DiffKind::Unchanged));
    }

    #[test]
    fn test_line_diff_empty_old() {
        let diff = line_diff("", "a\nb");
        assert_eq!(diff.len(), 2);
        assert!(diff.iter().all(|d| d.kind == DiffKind::Added));
    }

    #[test]
    fn test_line_diff_empty_new() {
        let diff = line_diff("a\nb", "");
        assert_eq!(diff.len(), 2);
        assert!(diff.iter().all(|d| d.kind == DiffKind::Removed));
    }

    #[test]
    fn test_line_diff_addition() {
        let diff = line_diff("a\nc", "a\nb\nc");
        assert_eq!(diff.len(), 3);
        assert_eq!(diff[0].kind, DiffKind::Unchanged);
        assert_eq!(diff[1].kind, DiffKind::Added);
        assert_eq!(diff[2].kind, DiffKind::Unchanged);
    }

    #[test]
    fn test_line_diff_removal() {
        let diff = line_diff("a\nb\nc", "a\nc");
        assert_eq!(diff.len(), 3);
        assert_eq!(diff[0].kind, DiffKind::Unchanged);
        assert_eq!(diff[1].kind, DiffKind::Removed);
        assert_eq!(diff[2].kind, DiffKind::Unchanged);
    }
}
