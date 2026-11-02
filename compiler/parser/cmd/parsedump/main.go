// Command parsedump parses Rust source files and prints their items
// (a sanity-check tool for the parser).
//
//	go run ./compiler/parser/cmd/parsedump file.rs...
package main

import (
	"fmt"
	"os"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"rustgo/compiler/parser"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: parsedump file.rs...")
		os.Exit(2)
	}
	failed := false
	for _, name := range os.Args[1:] {
		src, err := os.ReadFile(name)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			failed = true
			continue
		}
		fid := lexer.AddFile(name, string(src))
		crate, err := parser.ParseCrate(fid, string(src))
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			failed = true
			continue
		}
		fmt.Printf("%s: %d items\n", name, len(crate.Items))
		for _, it := range crate.Items {
			dump(it, 1)
		}
	}
	if failed {
		os.Exit(1)
	}
}

func dump(it *ast.Item, depth int) {
	ind := strings.Repeat("  ", depth)
	var attrs []string
	for _, a := range it.Attrs {
		attrs = append(attrs, "#["+a.Path+"]")
	}
	prefix := ind
	if len(attrs) > 0 {
		prefix += strings.Join(attrs, " ") + " "
	}
	switch k := it.Kind.(type) {
	case *ast.FnItem:
		fmt.Printf("%sfn %s%s (%d params)\n", prefix, it.Name.Name, ast.GenericsParamsString(k.Generics), len(k.Decl.Params))
	case *ast.StructItem:
		fmt.Printf("%sstruct %s%s (%d fields)\n", prefix, it.Name.Name, ast.GenericsParamsString(k.Generics), len(k.Fields))
	case *ast.UnionItem:
		fmt.Printf("%sunion %s\n", prefix, it.Name.Name)
	case *ast.EnumItem:
		fmt.Printf("%senum %s%s (%d variants)\n", prefix, it.Name.Name, ast.GenericsParamsString(k.Generics), len(k.Variants))
	case *ast.TraitItem:
		fmt.Printf("%strait %s%s\n", prefix, it.Name.Name, ast.GenericsParamsString(k.Generics))
		for _, sub := range k.Items {
			dump(sub, depth+1)
		}
	case *ast.ImplItem:
		tr := ""
		if k.Trait != nil {
			tr = ast.PathString(k.Trait) + " for "
			if k.Negative {
				tr = "!" + tr
			}
		}
		fmt.Printf("%simpl%s %s%s\n", prefix, ast.GenericsParamsString(k.Generics), tr, ast.TypeString(k.SelfTy))
		for _, sub := range k.Items {
			dump(sub, depth+1)
		}
	case *ast.TypeAliasItem:
		fmt.Printf("%stype %s\n", prefix, it.Name.Name)
	case *ast.ConstItem:
		fmt.Printf("%sconst %s\n", prefix, it.Name.Name)
	case *ast.StaticItem:
		fmt.Printf("%sstatic %s\n", prefix, it.Name.Name)
	case *ast.ModItem:
		fmt.Printf("%smod %s (inline=%v)\n", prefix, it.Name.Name, k.Inline)
		for _, sub := range k.Items {
			dump(sub, depth+1)
		}
	case *ast.UseItem:
		fmt.Printf("%suse ...\n", prefix)
	case *ast.ExternCrateItem:
		fmt.Printf("%sextern crate %s\n", prefix, it.Name.Name)
	case *ast.ExternBlockItem:
		fmt.Printf("%sextern %q block (%d items)\n", prefix, k.Abi, len(k.Items))
	case *ast.MacroRulesItem:
		fmt.Printf("%smacro_rules! %s\n", prefix, it.Name.Name)
	case *ast.MacCallItem:
		fmt.Printf("%s%s!(...)\n", prefix, ast.PathString(k.Mac.Path))
	default:
		fmt.Printf("%s%T %s\n", prefix, k, it.Name.Name)
	}
}
