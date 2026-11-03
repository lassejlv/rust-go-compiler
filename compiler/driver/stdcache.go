package driver

// The standard library is parsed and macro-expanded the same way on every
// run (its source is embedded in the compiler), which takes a good part of
// the compile time of small programs. The expanded crate is therefore
// cached on disk, together with the expander state it leaves (its
// macro_rules! definitions), in the user cache directory:
//
//	$RUSTGO_CACHE_DIR or os.UserCacheDir()/rustgo/std-<key>.bin
//
// The key identifies the compiler executable (its Go build ID, which
// changes with any change of the compiler or of the embedded std, or else
// a hash of the executable) and the cache format. A cache file is written
// to a temporary file and renamed into place (concurrent compilers never
// see a partial file), carries a checksum, and any problem reading it
// falls back to parsing std. RUSTGO_STD_CACHE=0 disables the cache; it is
// not used with --std.

import (
	"bytes"
	"crypto/sha256"
	"debug/elf"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"fmt"
	"hash/crc32"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"time"

	"rustgo/compiler/ast"
	"rustgo/compiler/expand"
	"rustgo/compiler/lexer"
)

// stdCacheFormat versions the cache file layout.
const stdCacheFormat = "rustgo-std-cache-1"

// stdCacheKeep is the number of cache files kept (one per compiler build;
// the least recently used ones are removed).
const stdCacheKeep = 8

var castagnoli = crc32.MakeTable(crc32.Castagnoli)

// loadExpandedStd returns the parsed and expanded std crate, expanded by
// exp (whose state then includes std's macros), from the cache if possible.
func loadExpandedStd(opts *Options, exp *expand.Expander) (*ast.Crate, error) {
	path := ""
	if opts.StdDir == "" && os.Getenv("RUSTGO_STD_CACHE") != "0" {
		path = stdCachePath()
	}
	idBase, fileBase, hygBase := ast.NextID(), len(lexer.Files), expand.HygieneMarks()
	if path != "" {
		if c := readStdCache(path, exp); c != nil {
			return c, nil
		}
	}
	c, err := loadStd(opts)
	if err != nil {
		return nil, fmt.Errorf("while loading std: %v", err)
	}
	if err := exp.ExpandCrate(c, "std"); err != nil {
		return nil, fmt.Errorf("while expanding std: %v", err)
	}
	if path != "" && !exp.UsedExternalInput() {
		writeStdCache(path, c, exp, idBase, fileBase, hygBase)
	}
	return c, nil
}

var (
	stdCacheOnce sync.Once
	stdCacheFile string
)

// stdCachePath returns the cache file for this compiler ("" if none).
func stdCachePath() string {
	stdCacheOnce.Do(func() {
		dir := os.Getenv("RUSTGO_CACHE_DIR")
		if dir == "" {
			d, err := os.UserCacheDir()
			if err != nil {
				return
			}
			dir = filepath.Join(d, "rustgo")
		}
		id := executableID()
		if id == nil {
			return
		}
		h := sha256.New()
		h.Write([]byte(stdCacheFormat))
		h.Write(id)
		stdCacheFile = filepath.Join(dir, "std-"+hex.EncodeToString(h.Sum(nil)[:12])+".bin")
	})
	return stdCacheFile
}

// executableID identifies the running compiler build: its Go build ID
// (ELF note) or else a hash of the executable.
func executableID() []byte {
	exe, err := os.Executable()
	if err != nil {
		return nil
	}
	if f, err := elf.Open(exe); err == nil {
		defer f.Close()
		if s := f.Section(".note.go.buildid"); s != nil {
			if data, err := s.Data(); err == nil && len(data) > 16 {
				return append([]byte("buildid:"), data...)
			}
		}
	}
	f, err := os.Open(exe)
	if err != nil {
		return nil
	}
	defer f.Close()
	h := sha256.New()
	if _, err := io.Copy(h, f); err != nil {
		return nil
	}
	return append([]byte("sha256:"), h.Sum(nil)...)
}

var stdCacheMagic = []byte("RGSTD\x00\x01\n")

// writeStdCache stores the expanded std crate c (and exp's std state);
// the base values are the global state from before std was loaded.
// Failures are silent: the cache is an optimization.
func writeStdCache(path string, c *ast.Crate, exp *expand.Expander, idBase ast.NodeID, fileBase int, hygBase uint32) {
	enc := ast.NewEncoder(idBase, fileBase, hygBase)
	enc.Uint(uint64(ast.NextID() - idBase))
	enc.Uint(uint64(expand.HygieneMarks() - hygBase))
	files := lexer.Files[fileBase:]
	enc.Uint(uint64(len(files)))
	for _, f := range files {
		enc.String(f.Name)
		enc.String(f.Src)
	}
	enc.Crate(c)
	exp.EncodeCrateState("std", enc)
	payload, err := enc.Bytes()
	if err != nil {
		return
	}
	var buf bytes.Buffer
	buf.Write(stdCacheMagic)
	var hdr [12]byte
	binary.LittleEndian.PutUint64(hdr[:8], uint64(len(payload)))
	binary.LittleEndian.PutUint32(hdr[8:], crc32.Checksum(payload, castagnoli))
	buf.Write(hdr[:])
	buf.Write(payload)
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return
	}
	tmp, err := os.CreateTemp(dir, "std-*.tmp")
	if err != nil {
		return
	}
	_, werr := tmp.Write(buf.Bytes())
	cerr := tmp.Close()
	if werr != nil || cerr != nil || os.Rename(tmp.Name(), path) != nil {
		os.Remove(tmp.Name())
		return
	}
	pruneStdCache(dir, path)
}

// readStdCache loads the std crate from the cache file into exp and the
// global state (source files, node IDs, hygiene marks). It returns nil
// (with the global state untouched) if the file is missing or invalid.
func readStdCache(path string, exp *expand.Expander) *ast.Crate {
	data, err := os.ReadFile(path)
	if err != nil || len(data) < len(stdCacheMagic)+12 || !bytes.Equal(data[:len(stdCacheMagic)], stdCacheMagic) {
		return nil
	}
	data = data[len(stdCacheMagic):]
	n := binary.LittleEndian.Uint64(data[:8])
	sum := binary.LittleEndian.Uint32(data[8:12])
	payload := data[12:]
	if uint64(len(payload)) != n || crc32.Checksum(payload, castagnoli) != sum {
		return nil
	}
	idBase, fileBase, hygBase := ast.NextID(), len(lexer.Files), expand.HygieneMarks()
	dec, err := ast.NewDecoder(payload, idBase, fileBase, hygBase)
	if err != nil {
		return nil
	}
	c, err := decodeStd(dec, exp)
	if err != nil {
		// undo what was registered
		lexer.Files = lexer.Files[:fileBase]
		ast.SetNextID(idBase)
		expand.SetHygieneMarks(hygBase)
		return nil
	}
	// mark the file used (pruneStdCache removes the least recently used)
	now := time.Now()
	os.Chtimes(path, now, now)
	return c
}

func decodeStd(dec *ast.Decoder, exp *expand.Expander) (*ast.Crate, error) {
	ids, err := dec.Uint()
	if err != nil {
		return nil, err
	}
	marks, err := dec.Uint()
	if err != nil {
		return nil, err
	}
	nf, err := dec.Uint()
	if err != nil {
		return nil, err
	}
	if nf > 1<<20 {
		return nil, errors.New("std cache: bad file count")
	}
	files := make([]*lexer.SourceFile, nf)
	for i := range files {
		name, err := dec.String()
		if err != nil {
			return nil, err
		}
		src, err := dec.String()
		if err != nil {
			return nil, err
		}
		files[i] = &lexer.SourceFile{Name: name, Src: src}
	}
	c, err := dec.Crate()
	if err != nil {
		return nil, err
	}
	commit, err := exp.DecodeCrateState("std", dec)
	if err != nil {
		return nil, err
	}
	if !dec.Done() {
		return nil, errors.New("std cache: trailing data")
	}
	commit()
	lexer.Files = append(lexer.Files, files...)
	ast.SetNextID(ast.NextID() + ast.NodeID(ids))
	expand.SetHygieneMarks(expand.HygieneMarks() + uint32(marks))
	return c, nil
}

// pruneStdCache removes all but the stdCacheKeep most recently used cache
// files of dir (never keep), and stale temporary files.
func pruneStdCache(dir, keep string) {
	ents, err := os.ReadDir(dir)
	if err != nil {
		return
	}
	type entry struct {
		path string
		mod  time.Time
	}
	var es []entry
	for _, e := range ents {
		name := e.Name()
		if !strings.HasPrefix(name, "std-") {
			continue
		}
		info, err := e.Info()
		if err != nil {
			continue
		}
		p := filepath.Join(dir, name)
		if strings.HasSuffix(name, ".tmp") {
			if time.Since(info.ModTime()) > time.Hour {
				os.Remove(p)
			}
			continue
		}
		if strings.HasSuffix(name, ".bin") && p != keep {
			es = append(es, entry{p, info.ModTime()})
		}
	}
	sort.Slice(es, func(i, j int) bool { return es[i].mod.After(es[j].mod) })
	for i, e := range es {
		if i >= stdCacheKeep-1 {
			os.Remove(e.path)
		}
	}
}
