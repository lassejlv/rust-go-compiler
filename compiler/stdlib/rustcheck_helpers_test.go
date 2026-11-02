package stdlib

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

func parserParseCrate(name, src string) (*ast.Crate, error) {
	return parser.ParseCrate(lexer.AddFile(name, src), src)
}
