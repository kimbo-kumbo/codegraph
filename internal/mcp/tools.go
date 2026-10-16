package mcp

import (
	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
)

func (s *Server) registerTools() {
	s.mcp.AddTool(
		mcp.NewTool("find_callers",
			mcp.WithDescription(
				"Find all callers of a symbol (direct and transitive). "+
					"Use when the user asks 'who calls X' or 'what depends on X'. "+
					"Returns a structured list of caller symbols with locations.",
			),
			mcp.WithString("symbol",
				mcp.Required(),
				mcp.Description("Symbol name, e.g. 'auth.Validate' or 'internal/auth.Validate'"),
			),
			mcp.WithBoolean("transitive",
				mcp.Description("Include transitive callers (callers of callers). Default: false"),
			),
			mcp.WithNumber("max_depth",
				mcp.Description("Max recursion depth. 0 = unlimited. Default: 0"),
			),
		),
		s.handleFindCallers,
	)

	s.mcp.AddTool(
		mcp.NewTool("impact_of_change",
			mcp.WithDescription(
				"Analyze the blast radius of changing a symbol. "+
					"Returns all functions, packages, and HTTP routes that will be affected. "+
					"Use when the user asks 'what breaks if I change X'.",
			),
			mcp.WithString("symbol",
				mcp.Required(),
				mcp.Description("Symbol name, e.g. 'auth.Validate'"),
			),
			mcp.WithNumber("max_depth",
				mcp.Description("Max recursion depth. 0 = unlimited. Default: 0"),
			),
			mcp.WithBoolean("include_tests",
				mcp.Description("Include test functions in the result. Default: false"),
			),
		),
		s.handleImpactOfChange,
	)

	s.mcp.AddTool(
		mcp.NewTool("find_dead_code",
			mcp.WithDescription(
				"Find code unreachable from entrypoints (main, HTTP routes, tests). "+
					"Use when the user asks about unused code or cleanup candidates. "+
					"By default returns all dead code including exported (public) "+
					"functions, since they might be used outside the package.",
			),
			mcp.WithBoolean("include_exported",
				mcp.Description(
					"Include exported (public) symbols in the result. "+
						"Default: true (recommended — public API might be used externally). "+
						"Set false to see only private dead functions.",
				),
			),
		),
		s.handleFindDeadCode,
	)

	s.mcp.AddTool(
		mcp.NewTool("check_architecture",
			mcp.WithDescription(
				"Check architectural rules (layering, forbidden dependencies, cycles). "+
					"Rules are passed as a list of ArchitectureRule objects. "+
					"Use when the user asks about architecture violations.",
			),
			mcp.WithArray("rules",
				mcp.Required(),
				mcp.Description("List of architecture rules. Each rule: {name, from, to, edge, forbid_cycles}"),
			),
		),
		s.handleCheckArchitecture,
	)

	s.mcp.AddTool(
		mcp.NewTool("find_type_smells",
			mcp.WithDescription(
				"Find type smells: interface{} in hot paths, reflection, global state. "+
					"Use when the user asks about type safety or performance smells.",
			),
			mcp.WithBoolean("only_hot_paths",
				mcp.Description("Only report smells in hot paths (high fan-in). Default: false"),
			),
			mcp.WithNumber("hot_path_threshold",
				mcp.Description("fan_in >= threshold counts as hot path. Default: 5"),
			),
		),
		s.handleFindTypeSmells,
	)

	s.mcp.AddTool(
		mcp.NewTool("find_complexity_patterns",
			mcp.WithDescription(
				"Find complexity patterns: nested loops, Contains/sort inside loops. "+
					"HEURISTIC — not a formal Big-O analysis. "+
					"Use when the user asks about performance hotspots.",
			),
			mcp.WithNumber("min_nesting_level",
				mcp.Description("Minimum loop nesting level. Default: 2"),
			),
			mcp.WithBoolean("include_recursion",
				mcp.Description("Include recursion heuristics. Default: false"),
			),
		),
		s.handleFindComplexityPatterns,
	)
	s.mcp.AddTool(
		mcp.NewTool("get_metrics",
			mcp.WithDescription(
				"Get package metrics: coupling (Ca/Ce), instability (I), abstractness (A), "+
					"distance from main sequence (D). "+
					"Use when the user asks about code quality or architecture metrics.",
			),
			mcp.WithString("package",
				mcp.Description("Package name. Empty = all packages."),
			),
		),
		s.handleGetMetrics,
	)
}

var _ = server.ServerTool{}