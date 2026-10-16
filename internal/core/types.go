// Package core — доменные типы и клиент к Rust-ядру через gRPC.
package core

type Language string

const (
	LanguageGo     Language = "go"
	LanguagePython Language = "python"
	LanguageRust   Language = "rust"
)

type NodeKind string

const (
	NodeKindFunction  NodeKind = "function"
	NodeKindMethod    NodeKind = "method"
	NodeKindType      NodeKind = "type"
	NodeKindInterface NodeKind = "interface"
	NodeKindPackage   NodeKind = "package"
	NodeKindRoute     NodeKind = "route"
	NodeKindTest      NodeKind = "test"
	NodeKindMain      NodeKind = "main"
)

type EdgeKind string

const (
	EdgeKindCalls      EdgeKind = "calls"
	EdgeKindImports    EdgeKind = "imports"
	EdgeKindImplements EdgeKind = "implements"
	EdgeKindReferences EdgeKind = "references"
)

type Severity string

const (
	SeverityInfo    Severity = "info"
	SeverityWarning Severity = "warning"
	SeverityError   Severity = "error"
)

type Location struct {
	File   string `json:"file"`
	Line   uint32 `json:"line"`
	Column uint32 `json:"column"`
}

type Symbol struct {
	ID            string   `json:"id"`
	Kind          NodeKind `json:"kind"`
	Name          string   `json:"name"`
	QualifiedName string   `json:"qualified_name"`
	Package       string   `json:"package"`
	Language      Language `json:"language"`
	Location      Location `json:"location"`
	Exported      bool     `json:"exported"`
	FanIn         uint32   `json:"fan_in"`
	FanOut        uint32   `json:"fan_out"`
}

type Diagnostic struct {
	Severity   Severity `json:"severity"`
	RuleID     string   `json:"rule_id"`
	Message    string   `json:"message"`
	Location   Location `json:"location"`
	Symbol     string   `json:"symbol,omitempty"`
	Suggestion string   `json:"suggestion,omitempty"`
}

type IndexRequest struct {
	ProjectPath string
	Languages   []Language
	Incremental bool
}

type IndexResult struct {
	FilesParsed  uint32   `json:"files_parsed"`
	NodesCreated uint32   `json:"nodes_created"`
	EdgesCreated uint32   `json:"edges_created"`
	DurationMs   uint64   `json:"duration_ms"`
	Errors       []string `json:"errors,omitempty"`
}

type FindCallersRequest struct {
	Symbol     string
	Transitive bool
	MaxDepth   uint32
}

type Callers struct {
	Symbol  Symbol   `json:"symbol"`
	Callers []Symbol `json:"callers"`
	Total   uint32   `json:"total"`
}

type ImpactRequest struct {
	Symbol       string
	MaxDepth     uint32
	IncludeTests bool
}

type Impact struct {
	Symbol           Symbol   `json:"symbol"`
	Affected         []Symbol `json:"affected"`
	AffectedPackages []string `json:"affected_packages"`
	AffectedRoutes   []Symbol `json:"affected_routes,omitempty"`
	TotalFunctions   uint32   `json:"total_functions"`
	TotalPackages    uint32   `json:"total_packages"`
}

type DeadCodeRequest struct {
	IncludeExported bool
	Languages       []Language
}

type DeadCodeResult struct {
	Dead  []Symbol `json:"dead"`
	Total uint32   `json:"total"`
}

type ArchitectureRule struct {
	Name         string   `yaml:"name"`
	Description  string   `yaml:"description,omitempty"`
	FromPattern  string   `yaml:"from"`
	ToPattern    string   `yaml:"to"`
	EdgeKind     EdgeKind `yaml:"edge"`
	ForbidCycles bool     `yaml:"forbid_cycles,omitempty"`
}

type CheckResult struct {
	Violations      []Diagnostic `json:"violations"`
	RulesChecked    uint32       `json:"rules_checked"`
	ViolationsCount uint32       `json:"violations_count"`
}

type TypeSmellsRequest struct {
	OnlyHotPaths     bool
	HotPathThreshold uint32
}

type TypeSmellsResult struct {
	Smells []Diagnostic `json:"smells"`
	Total  uint32       `json:"total"`
}

type ComplexityRequest struct {
	MinNestingLevel  uint32
	IncludeRecursion bool
}

type ComplexityResult struct {
	Patterns []Diagnostic `json:"patterns"`
	Total    uint32       `json:"total"`
}

type MetricsRequest struct {
	Package string
}

type PackageMetrics struct {
	Package      string   `json:"package"`
	Language     Language `json:"language"`
	FanIn        uint32   `json:"fan_in"`
	FanOut       uint32   `json:"fan_out"`
	Instability  float64  `json:"instability"`
	Abstractness float64  `json:"abstractness"`
	Distance     float64  `json:"distance"`
	NodesCount   uint32   `json:"nodes_count"`
	EdgesCount   uint32   `json:"edges_count"`
}

type MetricsResult struct {
	Packages []PackageMetrics `json:"packages"`
}