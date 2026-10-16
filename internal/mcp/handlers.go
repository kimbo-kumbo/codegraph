package mcp

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/kimbo-kumbo/codegraph/internal/core"
	"github.com/mark3labs/mcp-go/mcp"
)

func jsonResult(v any) (*mcp.CallToolResult, error) {
	data, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		return mcp.NewToolResultError(fmt.Sprintf("marshal: %v", err)), nil
	}
	return mcp.NewToolResultText(string(data)), nil
}


func (s *Server) handleFindCallers(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	symbol, err := req.RequireString("symbol")
	if err != nil {
		return mcp.NewToolResultError("missing required parameter: symbol"), nil
	}

	transitive := req.GetBool("transitive", false)
	maxDepth := uint32(req.GetInt("max_depth", 0))

	result, err := s.client.FindCallers(ctx, core.FindCallersRequest{
		Symbol:     symbol,
		Transitive: transitive,
		MaxDepth:   maxDepth,
	})
	if err != nil {
		return mcp.NewToolResultError(fmt.Sprintf("find_callers: %v", err)), nil
	}

	return jsonResult(result)
}

func (s *Server) handleImpactOfChange(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	symbol, err := req.RequireString("symbol")
	if err != nil {
		return mcp.NewToolResultError("missing required parameter: symbol"), nil
	}

	maxDepth := uint32(req.GetInt("max_depth", 0))
	includeTests := req.GetBool("include_tests", false)

	result, err := s.client.ImpactOfChange(ctx, core.ImpactRequest{
		Symbol:       symbol,
		MaxDepth:     maxDepth,
		IncludeTests: includeTests,
	})
	if err != nil {
		return mcp.NewToolResultError(fmt.Sprintf("impact_of_change: %v", err)), nil
	}

	return jsonResult(result)
}

func (s *Server) handleFindDeadCode(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	includeExported := req.GetBool("include_exported", true)

	result, err := s.client.FindDeadCode(ctx, core.DeadCodeRequest{
		IncludeExported: includeExported,
	})
	if err != nil {
		return mcp.NewToolResultError(fmt.Sprintf("find_dead_code: %v", err)), nil
	}

	return jsonResult(result)
}


func (s *Server) handleCheckArchitecture(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	rawRules, ok := req.GetArguments()["rules"]
	if !ok {
		return mcp.NewToolResultError("missing required parameter: rules"), nil
	}

	rules, err := parseRules(rawRules)
	if err != nil {
		return mcp.NewToolResultError(fmt.Sprintf("parse rules: %v", err)), nil
	}

	result, err := s.client.CheckArchitecture(ctx, rules)
	if err != nil {
		return mcp.NewToolResultError(fmt.Sprintf("check_architecture: %v", err)), nil
	}

	return jsonResult(result)
}

func parseRules(raw any) ([]core.ArchitectureRule, error) {
	arr, ok := raw.([]any)
	if !ok {
		return nil, fmt.Errorf("rules must be an array, got %T", raw)
	}
	out := make([]core.ArchitectureRule, 0, len(arr))
	for i, item := range arr {
		m, ok := item.(map[string]any)
		if !ok {
			return nil, fmt.Errorf("rule[%d] must be an object, got %T", i, item)
		}
		rule := core.ArchitectureRule{
			Name:        getString(m, "name"),
			Description: getString(m, "description"),
			FromPattern: getString(m, "from"),
			ToPattern:   getString(m, "to"),
			EdgeKind:    core.EdgeKind(getString(m, "edge")),
		}
		if b, ok := m["forbid_cycles"].(bool); ok {
			rule.ForbidCycles = b
		}
		out = append(out, rule)
	}
	return out, nil
}

func getString(m map[string]any, key string) string {
	if v, ok := m[key].(string); ok {
		return v
	}
	return ""
}


func (s *Server) handleFindTypeSmells(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	onlyHotPaths := req.GetBool("only_hot_paths", false)
	threshold := uint32(req.GetInt("hot_path_threshold", 5))

	result, err := s.client.FindTypeSmells(ctx, core.TypeSmellsRequest{
		OnlyHotPaths:     onlyHotPaths,
		HotPathThreshold: threshold,
	})
	if err != nil {
		return mcp.NewToolResultError(fmt.Sprintf("find_type_smells: %v", err)), nil
	}

	return jsonResult(result)
}

func (s *Server) handleFindComplexityPatterns(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	minNesting := uint32(req.GetInt("min_nesting_level", 2))
	includeRecursion := req.GetBool("include_recursion", false)

	result, err := s.client.FindComplexityPatterns(ctx, core.ComplexityRequest{
		MinNestingLevel:  minNesting,
		IncludeRecursion: includeRecursion,
	})
	if err != nil {
		return mcp.NewToolResultError(fmt.Sprintf("find_complexity_patterns: %v", err)), nil
	}

	return jsonResult(result)
}

func (s *Server) handleGetMetrics(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
	pkg := req.GetString("package", "")

	result, err := s.client.GetMetrics(ctx, core.MetricsRequest{Package: pkg})
	if err != nil {
		return mcp.NewToolResultError(fmt.Sprintf("get_metrics: %v", err)), nil
	}

	return jsonResult(result)
}
