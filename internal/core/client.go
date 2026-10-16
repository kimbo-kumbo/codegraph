package core

import (
	"context"
	"fmt"
	"time"

	"github.com/kimbo-kumbo/codegraph/internal/core/pb"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type Client interface {
	Index(ctx context.Context, req IndexRequest) (*IndexResult, error)
	FindCallers(ctx context.Context, req FindCallersRequest) (*Callers, error)
	ImpactOfChange(ctx context.Context, req ImpactRequest) (*Impact, error)
	FindDeadCode(ctx context.Context, req DeadCodeRequest) (*DeadCodeResult, error)
	CheckArchitecture(ctx context.Context, rules []ArchitectureRule) (*CheckResult, error)
	FindTypeSmells(ctx context.Context, req TypeSmellsRequest) (*TypeSmellsResult, error)
	FindComplexityPatterns(ctx context.Context, req ComplexityRequest) (*ComplexityResult, error)
	GetMetrics(ctx context.Context, req MetricsRequest) (*MetricsResult, error)
	Close() error
}

type grpcClient struct {
	conn   *grpc.ClientConn
	client pb.CodeGraphClient
}

func NewGRPCClient(addr string) (Client, error) {
	conn, err := grpc.NewClient(
		addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		return nil, fmt.Errorf("grpc.NewClient(%s): %w", addr, err)
	}
	return &grpcClient{
		conn:   conn,
		client: pb.NewCodeGraphClient(conn),
	}, nil
}

func (c *grpcClient) Close() error {
	return c.conn.Close()
}

func (c *grpcClient) Index(ctx context.Context, req IndexRequest) (*IndexResult, error) {
	pbReq := &pb.IndexRequest{
		ProjectPath: req.ProjectPath,
		Languages:   toPBLanguages(req.Languages),
		Incremental: req.Incremental,
	}
	resp, err := c.client.Index(ctx, pbReq)
	if err != nil {
		return nil, fmt.Errorf("Index(%s): %w", req.ProjectPath, err)
	}
	return &IndexResult{
		FilesParsed:  resp.GetFilesParsed(),
		NodesCreated: resp.GetNodesCreated(),
		EdgesCreated: resp.GetEdgesCreated(),
		DurationMs:   resp.GetDurationMs(),
		Errors:       resp.GetErrors(),
	}, nil
}

func (c *grpcClient) FindCallers(ctx context.Context, req FindCallersRequest) (*Callers, error) {
	resp, err := c.client.FindCallers(ctx, &pb.FindCallersRequest{
		Symbol:     req.Symbol,
		Transitive: req.Transitive,
		MaxDepth:   req.MaxDepth,
	})
	if err != nil {
		return nil, fmt.Errorf("FindCallers(%s): %w", req.Symbol, err)
	}
	return &Callers{
		Symbol:  fromPBNode(resp.GetSymbol()),
		Callers: fromPBNodes(resp.GetCallers()),
		Total:   resp.GetTotal(),
	}, nil
}

func (c *grpcClient) ImpactOfChange(ctx context.Context, req ImpactRequest) (*Impact, error) {
	resp, err := c.client.ImpactOfChange(ctx, &pb.ImpactOfChangeRequest{
		Symbol:       req.Symbol,
		MaxDepth:     req.MaxDepth,
		IncludeTests: req.IncludeTests,
	})
	if err != nil {
		return nil, fmt.Errorf("ImpactOfChange(%s): %w", req.Symbol, err)
	}
	return &Impact{
		Symbol:           fromPBNode(resp.GetSymbol()),
		Affected:         fromPBNodes(resp.GetAffected()),
		AffectedPackages: resp.GetAffectedPackages(),
		AffectedRoutes:   fromPBNodes(resp.GetAffectedRoutes()),
		TotalFunctions:   resp.GetTotalFunctions(),
		TotalPackages:    resp.GetTotalPackages(),
	}, nil
}

func (c *grpcClient) FindDeadCode(ctx context.Context, req DeadCodeRequest) (*DeadCodeResult, error) {
	resp, err := c.client.FindDeadCode(ctx, &pb.FindDeadCodeRequest{
		IncludeExported: req.IncludeExported,
		Languages:       toPBLanguages(req.Languages),
	})
	if err != nil {
		return nil, fmt.Errorf("FindDeadCode: %w", err)
	}
	return &DeadCodeResult{
		Dead:  fromPBNodes(resp.GetDead()),
		Total: resp.GetTotal(),
	}, nil
}

func (c *grpcClient) CheckArchitecture(ctx context.Context, rules []ArchitectureRule) (*CheckResult, error) {
	pbRules := make([]*pb.ArchitectureRule, 0, len(rules))
	for _, r := range rules {
		pbRules = append(pbRules, &pb.ArchitectureRule{
			Name:         r.Name,
			Description:  r.Description,
			FromPattern:  r.FromPattern,
			ToPattern:    r.ToPattern,
			EdgeKind:     toPBEdgeKind(r.EdgeKind),
			ForbidCycles: r.ForbidCycles,
		})
	}
	resp, err := c.client.CheckArchitecture(ctx, &pb.CheckArchitectureRequest{Rules: pbRules})
	if err != nil {
		return nil, fmt.Errorf("CheckArchitecture: %w", err)
	}
	return &CheckResult{
		Violations:      fromPBDiagnostics(resp.GetViolations()),
		RulesChecked:    resp.GetRulesChecked(),
		ViolationsCount: resp.GetViolationsCount(),
	}, nil
}

func (c *grpcClient) FindTypeSmells(ctx context.Context, req TypeSmellsRequest) (*TypeSmellsResult, error) {
	resp, err := c.client.FindTypeSmells(ctx, &pb.FindTypeSmellsRequest{
		OnlyHotPaths:     req.OnlyHotPaths,
		HotPathThreshold: req.HotPathThreshold,
	})
	if err != nil {
		return nil, fmt.Errorf("FindTypeSmells: %w", err)
	}
	return &TypeSmellsResult{
		Smells: fromPBDiagnostics(resp.GetSmells()),
		Total:  resp.GetTotal(),
	}, nil
}

func (c *grpcClient) FindComplexityPatterns(ctx context.Context, req ComplexityRequest) (*ComplexityResult, error) {
	resp, err := c.client.FindComplexityPatterns(ctx, &pb.FindComplexityPatternsRequest{
		MinNestingLevel:  req.MinNestingLevel,
		IncludeRecursion: req.IncludeRecursion,
	})
	if err != nil {
		return nil, fmt.Errorf("FindComplexityPatterns: %w", err)
	}
	return &ComplexityResult{
		Patterns: fromPBDiagnostics(resp.GetPatterns()),
		Total:    resp.GetTotal(),
	}, nil
}

func (c *grpcClient) GetMetrics(ctx context.Context, req MetricsRequest) (*MetricsResult, error) {
	resp, err := c.client.GetMetrics(ctx, &pb.GetMetricsRequest{Package: req.Package})
	if err != nil {
		return nil, fmt.Errorf("GetMetrics(%s): %w", req.Package, err)
	}
	out := &MetricsResult{Packages: make([]PackageMetrics, 0, len(resp.GetPackages()))}
	for _, m := range resp.GetPackages() {
		out.Packages = append(out.Packages, PackageMetrics{
			Package:      m.GetPackage(),
			Language:     fromPBLanguage(m.GetLanguage()),
			FanIn:        m.GetFanIn(),
			FanOut:       m.GetFanOut(),
			Instability:  m.GetInstability(),
			Abstractness: m.GetAbstractness(),
			Distance:     m.GetDistance(),
			NodesCount:   m.GetNodesCount(),
			EdgesCount:   m.GetEdgesCount(),
		})
	}
	return out, nil
}

func fromPBNode(n *pb.Node) Symbol {
	if n == nil {
		return Symbol{}
	}
	loc := Location{}
	if l := n.GetLocation(); l != nil {
		loc = Location{File: l.GetFile(), Line: l.GetLine(), Column: l.GetColumn()}
	}
	return Symbol{
		ID:            n.GetId(),
		Kind:          fromPBNodeKind(n.GetKind()),
		Name:          n.GetName(),
		QualifiedName: n.GetQualifiedName(),
		Package:       n.GetPackage(),
		Language:      fromPBLanguage(n.GetLanguage()),
		Location:      loc,
		Exported:      n.GetExported(),
		FanIn:         n.GetFanIn(),
		FanOut:        n.GetFanOut(),
	}
}

func fromPBNodes(ns []*pb.Node) []Symbol {
	out := make([]Symbol, 0, len(ns))
	for _, n := range ns {
		out = append(out, fromPBNode(n))
	}
	return out
}

func fromPBDiagnostics(ds []*pb.Diagnostic) []Diagnostic {
	out := make([]Diagnostic, 0, len(ds))
	for _, d := range ds {
		loc := Location{}
		if l := d.GetLocation(); l != nil {
			loc = Location{File: l.GetFile(), Line: l.GetLine(), Column: l.GetColumn()}
		}
		out = append(out, Diagnostic{
			Severity:   fromPBSeverity(d.GetSeverity()),
			RuleID:     d.GetRuleId(),
			Message:    d.GetMessage(),
			Location:   loc,
			Symbol:     d.GetSymbol(),
			Suggestion: d.GetSuggestion(),
		})
	}
	return out
}

func toPBLanguages(ls []Language) []pb.Language {
	out := make([]pb.Language, 0, len(ls))
	for _, l := range ls {
		out = append(out, toPBLanguage(l))
	}
	return out
}

func toPBLanguage(l Language) pb.Language {
	switch l {
	case LanguageGo:
		return pb.Language_LANGUAGE_GO
	case LanguagePython:
		return pb.Language_LANGUAGE_PYTHON
	case LanguageRust:
		return pb.Language_LANGUAGE_RUST
	default:
		return pb.Language_LANGUAGE_UNSPECIFIED
	}
}

func toPBEdgeKind(k EdgeKind) pb.EdgeKind {
	switch k {
	case EdgeKindCalls:
		return pb.EdgeKind_EDGE_KIND_CALLS
	case EdgeKindImports:
		return pb.EdgeKind_EDGE_KIND_IMPORTS
	case EdgeKindImplements:
		return pb.EdgeKind_EDGE_KIND_IMPLEMENTS
	case EdgeKindReferences:
		return pb.EdgeKind_EDGE_KIND_REFERENCES
	default:
		return pb.EdgeKind_EDGE_KIND_UNSPECIFIED
	}
}

func fromPBLanguage(l pb.Language) Language {
	switch l {
	case pb.Language_LANGUAGE_GO:
		return LanguageGo
	case pb.Language_LANGUAGE_PYTHON:
		return LanguagePython
	case pb.Language_LANGUAGE_RUST:
		return LanguageRust
	default:
		return ""
	}
}

func fromPBNodeKind(k pb.NodeKind) NodeKind {
	switch k {
	case pb.NodeKind_NODE_KIND_FUNCTION:
		return NodeKindFunction
	case pb.NodeKind_NODE_KIND_METHOD:
		return NodeKindMethod
	case pb.NodeKind_NODE_KIND_TYPE:
		return NodeKindType
	case pb.NodeKind_NODE_KIND_INTERFACE:
		return NodeKindInterface
	case pb.NodeKind_NODE_KIND_PACKAGE:
		return NodeKindPackage
	case pb.NodeKind_NODE_KIND_ROUTE:
		return NodeKindRoute
	case pb.NodeKind_NODE_KIND_TEST:
		return NodeKindTest
	case pb.NodeKind_NODE_KIND_MAIN:
		return NodeKindMain
	default:
		return ""
	}
}

func fromPBSeverity(s pb.Diagnostic_Severity) Severity {
	switch s {
	case pb.Diagnostic_SEVERITY_INFO:
		return SeverityInfo
	case pb.Diagnostic_SEVERITY_WARNING:
		return SeverityWarning
	case pb.Diagnostic_SEVERITY_ERROR:
		return SeverityError
	default:
		return ""
	}
}

type mockClient struct {
	IndexResult      *IndexResult
	Callers          *Callers
	Impact           *Impact
	DeadCode         *DeadCodeResult
	CheckResult      *CheckResult
	TypeSmells       *TypeSmellsResult
	ComplexityResult *ComplexityResult
	MetricsResult    *MetricsResult
}

func NewMockClient() Client {
	return &mockClient{
		IndexResult: &IndexResult{
			FilesParsed:  10,
			NodesCreated: 100,
			EdgesCreated: 250,
			DurationMs:   42,
		},
		Callers: &Callers{
			Symbol: Symbol{
				ID:            "go:auth:Validate",
				Kind:          NodeKindFunction,
				Name:          "Validate",
				QualifiedName: "auth.Validate",
				Package:       "auth",
				Language:      LanguageGo,
				Location:      Location{File: "internal/auth/auth.go", Line: 15, Column: 1},
				Exported:      true,
				FanIn:         2,
				FanOut:        1,
			},
			Callers: []Symbol{
				{
					ID:            "go:auth:Login",
					Kind:          NodeKindFunction,
					Name:          "Login",
					QualifiedName: "auth.Login",
					Package:       "auth",
					Language:      LanguageGo,
					Location:      Location{File: "internal/auth/auth.go", Line: 30, Column: 1},
					Exported:      true,
					FanIn:         1,
					FanOut:        2,
				},
				{
					ID:            "go:api:HandleLogin",
					Kind:          NodeKindFunction,
					Name:          "HandleLogin",
					QualifiedName: "api.HandleLogin",
					Package:       "api",
					Language:      LanguageGo,
					Location:      Location{File: "internal/api/handlers.go", Line: 42, Column: 1},
					Exported:      true,
					FanIn:         0,
					FanOut:        3,
				},
			},
			Total: 2,
		},
		Impact: &Impact{
			Symbol: Symbol{
				ID:            "go:auth:Validate",
				Kind:          NodeKindFunction,
				Name:          "Validate",
				QualifiedName: "auth.Validate",
				Package:       "auth",
				Language:      LanguageGo,
				Location:      Location{File: "internal/auth/auth.go", Line: 15, Column: 1},
				Exported:      true,
				FanIn:         2,
				FanOut:        1,
			},
			Affected: []Symbol{
				{
					ID:            "go:auth:Login",
					Kind:          NodeKindFunction,
					Name:          "Login",
					QualifiedName: "auth.Login",
					Package:       "auth",
					Language:      LanguageGo,
					Location:      Location{File: "internal/auth/auth.go", Line: 30, Column: 1},
					Exported:      true,
					FanIn:         1,
					FanOut:        2,
				},
				{
					ID:            "go:api:HandleLogin",
					Kind:          NodeKindFunction,
					Name:          "HandleLogin",
					QualifiedName: "api.HandleLogin",
					Package:       "api",
					Language:      LanguageGo,
					Location:      Location{File: "internal/api/handlers.go", Line: 42, Column: 1},
					Exported:      true,
					FanIn:         0,
					FanOut:        3,
				},
			},
			AffectedPackages: []string{"auth", "api"},
			AffectedRoutes: []Symbol{
				{
					ID:            "go:api:POST:/login",
					Kind:          NodeKindRoute,
					Name:          "POST /login",
					QualifiedName: "POST /login",
					Package:       "api",
					Language:      LanguageGo,
					Location:      Location{File: "internal/api/handlers.go", Line: 40, Column: 1},
					Exported:      true,
				},
			},
			TotalFunctions: 2,
			TotalPackages:  2,
		},
		DeadCode: &DeadCodeResult{
			Dead: []Symbol{
				{
					ID:            "go:legacy:OldHandler",
					Kind:          NodeKindFunction,
					Name:          "OldHandler",
					QualifiedName: "legacy.OldHandler",
					Package:       "legacy",
					Language:      LanguageGo,
					Location:      Location{File: "internal/legacy/handler.go", Line: 10, Column: 1},
					Exported:      false,
				},
			},
			Total: 1,
		},
		CheckResult: &CheckResult{
			Violations: []Diagnostic{
				{
					Severity: SeverityError,
					RuleID:   "architecture.layers",
					Message:  "handlers must not access DB directly",
					Location: Location{File: "internal/handlers/user.go", Line: 42, Column: 5},
					Symbol:   "handlers.GetUser",
				},
			},
			RulesChecked:    2,
			ViolationsCount: 1,
		},
		TypeSmells: &TypeSmellsResult{
			Smells: []Diagnostic{
				{
					Severity:   SeverityWarning,
					RuleID:     "type-smells.interface{}",
					Message:    "interface{} in public signature of hot path",
					Symbol:     "parser.Parse",
					Location:   Location{File: "internal/parser/json.go", Line: 25, Column: 1},
					Suggestion: "замени на generic или конкретный тип",
				},
			},
			Total: 1,
		},
		ComplexityResult: &ComplexityResult{
			Patterns: []Diagnostic{
				{
					Severity: SeverityWarning,
					RuleID:   "complexity.nested-loops",
					Message:  "potential O(n²): nested loops with slices.Contains inside",
					Symbol:   "processor.ProcessAll",
					Location: Location{File: "internal/processor/main.go", Line: 100, Column: 3},
				},
			},
			Total: 1,
		},
		MetricsResult: &MetricsResult{
			Packages: []PackageMetrics{
				{Package: "auth", Language: LanguageGo, FanIn: 3, FanOut: 5, Instability: 0.625, Abstractness: 0.2, Distance: 0.175, NodesCount: 8, EdgesCount: 12},
				{Package: "api", Language: LanguageGo, FanIn: 1, FanOut: 4, Instability: 0.8, Abstractness: 0.1, Distance: 0.3, NodesCount: 5, EdgesCount: 9},
			},
		},
	}
}

func (m *mockClient) Index(_ context.Context, _ IndexRequest) (*IndexResult, error) {
	time.Sleep(10 * time.Millisecond)
	return m.IndexResult, nil
}

func (m *mockClient) FindCallers(_ context.Context, _ FindCallersRequest) (*Callers, error) {
	return m.Callers, nil
}

func (m *mockClient) ImpactOfChange(_ context.Context, _ ImpactRequest) (*Impact, error) {
	return m.Impact, nil
}

func (m *mockClient) FindDeadCode(_ context.Context, _ DeadCodeRequest) (*DeadCodeResult, error) {
	return m.DeadCode, nil
}

func (m *mockClient) CheckArchitecture(_ context.Context, _ []ArchitectureRule) (*CheckResult, error) {
	return m.CheckResult, nil
}

func (m *mockClient) FindTypeSmells(_ context.Context, _ TypeSmellsRequest) (*TypeSmellsResult, error) {
	return m.TypeSmells, nil
}

func (m *mockClient) FindComplexityPatterns(_ context.Context, _ ComplexityRequest) (*ComplexityResult, error) {
	return m.ComplexityResult, nil
}

func (m *mockClient) GetMetrics(_ context.Context, _ MetricsRequest) (*MetricsResult, error) {
	return m.MetricsResult, nil
}

func (m *mockClient) Close() error { return nil }
