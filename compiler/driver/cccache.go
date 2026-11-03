package driver

// Object file cache. The generated C is deterministic, so rebuilding an
// unchanged program (or the unchanged build script / proc-macro server of
// a cargo package) produces the same translation units: their object files
// are cached on disk, keyed by a hash of the C compiler (path, size and
// modification time of its executable), its flags and the unit's source:
//
//	$RUSTGO_CACHE_DIR or os.UserCacheDir()/rustgo/cc/<key>.o
//
// Only compilation (`cc -c`) is cached, never linking (libraries named by
// link arguments may change without changing the key). Entries are written
// to a temporary file and renamed into place; the least recently used ones
// are removed when the cache outgrows ccCacheMaxBytes.
// RUSTGO_CC_CACHE=0 disables the cache.

import (
	"crypto/sha256"
	"encoding/hex"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"
)

const ccCacheFormat = "rustgo-cc-cache-1"

// ccCacheMaxBytes bounds the size of the object cache
// (RUSTGO_CC_CACHE_MB overrides it).
const ccCacheMaxBytes = 256 << 20

type ccCache struct {
	dir  string
	salt []byte // compiler identity and flags
	mu   sync.Mutex
	put  bool // something was added (prune afterwards)
}

// newCCCache returns the object cache for compiling with cc and cflags, or
// nil when it is disabled or unavailable.
func newCCCache(cc string, cflags []string) *ccCache {
	if os.Getenv("RUSTGO_CC_CACHE") == "0" {
		return nil
	}
	dir := os.Getenv("RUSTGO_CACHE_DIR")
	if dir == "" {
		d, err := os.UserCacheDir()
		if err != nil {
			return nil
		}
		dir = filepath.Join(d, "rustgo")
	}
	dir = filepath.Join(dir, "cc")
	path, err := exec.LookPath(cc)
	if err != nil {
		return nil
	}
	if p, err := filepath.EvalSymlinks(path); err == nil {
		path = p
	}
	info, err := os.Stat(path)
	if err != nil {
		return nil
	}
	h := sha256.New()
	io.WriteString(h, ccCacheFormat+"\x00"+path+"\x00"+strconv.FormatInt(info.Size(), 10)+"\x00"+strconv.FormatInt(info.ModTime().UnixNano(), 10)+"\x00")
	for _, f := range cflags {
		io.WriteString(h, f+"\x00")
	}
	return &ccCache{dir: dir, salt: h.Sum(nil)}
}

// path returns the cache file of a translation unit.
func (c *ccCache) path(src string) string {
	h := sha256.New()
	h.Write(c.salt)
	io.WriteString(h, src)
	return filepath.Join(c.dir, hex.EncodeToString(h.Sum(nil)[:16])+".o")
}

// get copies the cached object of src to obj and reports whether it was
// there.
func (c *ccCache) get(src, obj string) bool {
	p := c.path(src)
	if err := copyFile(p, obj); err != nil {
		return false
	}
	now := time.Now()
	os.Chtimes(p, now, now) // (pruning removes the least recently used)
	return true
}

// add stores obj as the object of src. Failures are silent: the cache is
// an optimization.
func (c *ccCache) add(src, obj string) {
	if err := os.MkdirAll(c.dir, 0o755); err != nil {
		return
	}
	tmp, err := os.CreateTemp(c.dir, "o-*.tmp")
	if err != nil {
		return
	}
	tmp.Close()
	defer os.Remove(tmp.Name()) // (fails once renamed)
	if copyFile(obj, tmp.Name()) != nil || os.Rename(tmp.Name(), c.path(src)) != nil {
		return
	}
	c.mu.Lock()
	c.put = true
	c.mu.Unlock()
}

func copyFile(from, to string) error {
	in, err := os.Open(from)
	if err != nil {
		return err
	}
	defer in.Close()
	out, err := os.Create(to)
	if err != nil {
		return err
	}
	if _, err := io.Copy(out, in); err != nil {
		out.Close()
		return err
	}
	return out.Close()
}

// prune removes the least recently used objects while the cache holds
// more than its size limit, and stale temporary files.
func (c *ccCache) prune() {
	if !c.put {
		return
	}
	max := int64(ccCacheMaxBytes)
	if s := os.Getenv("RUSTGO_CC_CACHE_MB"); s != "" {
		if n, err := strconv.ParseInt(s, 10, 64); err == nil && n >= 0 {
			max = n << 20
		}
	}
	ents, err := os.ReadDir(c.dir)
	if err != nil {
		return
	}
	type entry struct {
		path string
		size int64
		mod  time.Time
	}
	var es []entry
	var total int64
	for _, e := range ents {
		info, err := e.Info()
		if err != nil {
			continue
		}
		p := filepath.Join(c.dir, e.Name())
		if strings.HasSuffix(e.Name(), ".tmp") {
			if time.Since(info.ModTime()) > time.Hour {
				os.Remove(p)
			}
			continue
		}
		if strings.HasSuffix(e.Name(), ".o") {
			es = append(es, entry{p, info.Size(), info.ModTime()})
			total += info.Size()
		}
	}
	if total <= max {
		return
	}
	sort.Slice(es, func(i, j int) bool { return es[i].mod.Before(es[j].mod) })
	for _, e := range es {
		if total <= max {
			break
		}
		if os.Remove(e.path) == nil {
			total -= e.size
		}
	}
}
