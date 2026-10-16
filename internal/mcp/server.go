// Package mcp — MCP-сервер codegraph.
package mcp

import (
	"context"
	"fmt"
	"os"

	"github.com/kimbo-kumbo/codegraph/internal/core"
	"github.com/mark3labs/mcp-go/server"
)

type Server struct {
	mcp    *server.MCPServer
	client core.Client
}

func NewServer(client core.Client) *Server {
	s := server.NewMCPServer(
		"codegraph",
		"0.1.0",
		server.WithToolCapabilities(false),
		server.WithRecovery(),
	)
	srv := &Server{
		mcp:    s,
		client: client,
	}
	srv.registerTools()
	return srv
}

func (s *Server) Serve(ctx context.Context) error {
	stdio := server.NewStdioServer(s.mcp)
	fmt.Fprintln(os.Stderr, "codegraph MCP server started (stdio)")
	return stdio.Listen(ctx, os.Stdin, os.Stdout)
}
