//! Эвристики на O(n²) в Go-коде.
use codegraph_ir::{Location, Severity};
use tree_sitter::{Node as TsNode, Parser};
use crate::walker::Walker;
use crate::{ParseError, Result};

#[derive(Debug, Clone)]
pub struct ComplexityPattern {
    pub severity: Severity,
    pub rule_id: String,
    pub message: String,
    pub location: Location,
    pub symbol: String,
    pub suggestion: String,
}

#[derive(Debug, Clone)]
pub struct ComplexityOptions {
    pub min_nesting_level: u32,
    pub include_recursion: bool,
}

impl Default for ComplexityOptions {
    fn default() -> Self {
        Self {
            min_nesting_level: 2,
            include_recursion: false,
        }
    }
}

pub fn find_patterns(
    source: &str,
    rel_path: &str,
    options: &ComplexityOptions,
) -> Result<Vec<ComplexityPattern>> {
    let tree = parse_tree(source)?;
    let root = tree.root_node();
    let walker = Walker::new(source, rel_path);

    let mut patterns = Vec::new();
    let mut state = WalkState::default();

    walk_node(root, &walker, &mut state, options, &mut patterns);

    Ok(patterns)
}

#[derive(Default)]
struct WalkState {
    loop_stack: Vec<(usize, usize)>,
    current_function: String,
    function_end: usize,
}

fn walk_node(
    node: TsNode,
    walker: &Walker,
    state: &mut WalkState,
    options: &ComplexityOptions,
    patterns: &mut Vec<ComplexityPattern>,
) {
    let kind = node.kind();

    if kind == "function_declaration" || kind == "method_declaration" {
        if let Some(name) = function_name(node, walker) {
            let prev = std::mem::replace(&mut state.current_function, name);
            let prev_end = state.function_end;
            state.function_end = node.byte_range().end;

            walk_children(node, walker, state, options, patterns);

            state.current_function = prev;
            state.function_end = prev_end;
            return;
        }
    }

    if kind == "call_expression" && !state.loop_stack.is_empty() {
        check_dangerous_call_inside_loop(node, walker, state, patterns);
    }

    if is_loop(kind) {
        let range = node.byte_range();
        let depth = state.loop_stack.len() as u32 + 1;

        if depth >= options.min_nesting_level {
            emit_nested_loop_pattern(node, depth, walker, state, patterns);
        }

        state.loop_stack.push((range.start, range.end));
        walk_children(node, walker, state, options, patterns);
        state.loop_stack.pop();
        return;
    }

    walk_children(node, walker, state, options, patterns);
}

fn walk_children(
    node: TsNode,
    walker: &Walker,
    state: &mut WalkState,
    options: &ComplexityOptions,
    patterns: &mut Vec<ComplexityPattern>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_node(child, walker, state, options, patterns);
    }
}

fn is_loop(kind: &str) -> bool {
    matches!(kind, "for_statement")
}

fn emit_nested_loop_pattern(
    node: TsNode,
    depth: u32,
    walker: &Walker,
    state: &WalkState,
    patterns: &mut Vec<ComplexityPattern>,
) {
    let (line, column) = walker.position(node);
    let location = Location::new(walker.rel_path, line, column);

    patterns.push(ComplexityPattern {
        severity: Severity::Warning,
        rule_id: "complexity.nested-loops".to_string(),
        message: format!(
            "potential O(n^{}): {} nested loops",
            depth, depth
        ),
        location,
        symbol: state.current_function.clone(),
        suggestion: "проверь, можно ли заменить внутренний цикл на map/set для O(1) lookup"
            .to_string(),
    });
}

fn check_dangerous_call_inside_loop(
    node: TsNode,
    walker: &Walker,
    state: &WalkState,
    patterns: &mut Vec<ComplexityPattern>,
) {
    if node.kind() != "call_expression" {
        return;
    }

    let Some(function) = node.child_by_field_name("function") else {
        return;
    };

    let call_name = match function.kind() {
        "identifier" => walker.text(function).to_string(),
        "selector_expression" => {
            let pkg = function
                .child_by_field_name("operand")
                .map(|n| walker.text(n).to_string())
                .unwrap_or_default();
            let method = function
                .child_by_field_name("field")
                .map(|n| walker.text(n).to_string())
                .unwrap_or_default();
            format!("{}.{}", pkg, method)
        }
        _ => return,
    };

    let dangerous = match call_name.as_str() {
        "slices.Contains" | "strings.Contains" | "strings.Index" | "strings.IndexByte" => {
            Some("O(n) поиск внутри цикла → возможный O(n²)")
        }
        "sort.Slice" | "sort.Strings" | "sort.Ints" | "sort.SliceStable" => {
            Some("O(n log n) сортировка внутри цикла → возможный O(n² log n)")
        }
        _ => None,
    };

    let Some(note) = dangerous else {
        return;
    };

    let (line, column) = walker.position(node);
    let location = Location::new(walker.rel_path, line, column);

    patterns.push(ComplexityPattern {
        severity: Severity::Warning,
        rule_id: "complexity.expensive-call-in-loop".to_string(),
        message: format!("{}: {}", call_name, note),
        location,
        symbol: state.current_function.clone(),
        suggestion: match call_name.as_str() {
            "slices.Contains" => "замени на map[T]struct{} для O(1) lookup".to_string(),
            "strings.Contains" => "рассмотри индекс или другую структуру".to_string(),
            _ if call_name.starts_with("sort.") => "вынеси сортировку из цикла".to_string(),
            _ => "проверь вручную".to_string(),
        },
    });
}

fn parse_tree(source: &str) -> Result<tree_sitter::Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .map_err(|e| ParseError::Parse(e.to_string()))?;

    parser
        .parse(source, None)
        .ok_or_else(|| ParseError::Parse("tree-sitter returned None".into()))
}

fn function_name(node: TsNode, walker: &Walker) -> Option<String> {
    node.child_by_field_name("name")
        .map(|n| walker.text(n).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NESTED_LOOPS: &str = r#"
package processor

import "slices"

func ProcessAll(items []string, targets []string) []string {
	var result []string
	for _, item := range items {
		for _, target := range targets {
			if slices.Contains(targets, item) {
				result = append(result, item+":"+target)
			}
		}
	}
	return result
}
"#;

    const SINGLE_LOOP: &str = r#"
package simple

func Sum(items []int) int {
	total := 0
	for _, x := range items {
		total += x
	}
	return total
}
"#;

    #[test]
    fn test_nested_loops_detected() {
        let patterns = find_patterns(
            NESTED_LOOPS,
            "processor/processor.go",
            &ComplexityOptions::default(),
        )
        .unwrap();
        assert!(patterns.len() >= 1, "no patterns found");

        let rules: Vec<&str> = patterns.iter().map(|p| p.rule_id.as_str()).collect();
        assert!(
            rules.iter().any(|r| *r == "complexity.nested-loops"),
            "no nested-loops pattern: {:?}",
            rules
        );
    }

    #[test]
    fn test_single_loop_no_pattern() {
        let patterns = find_patterns(
            SINGLE_LOOP,
            "simple/simple.go",
            &ComplexityOptions::default(),
        )
        .unwrap();

        let nested = patterns
            .iter()
            .filter(|p| p.rule_id == "complexity.nested-loops")
            .count();
        assert_eq!(nested, 0, "false positive: nested-loops in single loop");
    }

    #[test]
    fn test_min_nesting_level_respected() {
        let opts = ComplexityOptions {
            min_nesting_level: 3,
            include_recursion: false,
        };
        let patterns = find_patterns(NESTED_LOOPS, "processor/processor.go", &opts).unwrap();

        let nested = patterns
            .iter()
            .filter(|p| p.rule_id == "complexity.nested-loops")
            .count();
        assert_eq!(nested, 0);
    }
}