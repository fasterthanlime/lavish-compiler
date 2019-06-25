package main

import (
	"fmt"
	"log"
	"html/template"
	"os"
	"path/filepath"

	"github.com/magefile/mage/sh"
	"github.com/otiai10/copy"
)

// A build step that requires additional params, or platform specific steps for example
func main() {
	must(TestCodegen())
}

func TestCodegen() error {
	args := os.Args[1:]
	singleTest := ""
	if len(args) > 0 {
		singleTest = args[0]
		log.Printf("Will run single test '%s'\n", singleTest)
	}

	cwd, err := os.Getwd()
	must(err)

	testsDir := filepath.Join(cwd, "codegen-tests")
	_, err = os.Stat(testsDir)
	must(err)

	testsDirHandle, err := os.Open(testsDir)
	must(err)
	testNames, err := testsDirHandle.Readdirnames(-1)
	must(err)
	must(testsDirHandle.Close())

	log.Println("Building lavish compiler")
	must(sh.RunV("cargo", "build", "--manifest-path", "../Cargo.toml"))

	log.Println("Running codegen tests...")

	tmpDir := filepath.Join(cwd, ".tmp")

	cargoTargetDir := filepath.Join(tmpDir, "target")
	os.Setenv("CARGO_TARGET_DIR", cargoTargetDir)

	harnessDir := filepath.Join(tmpDir, "harness")

	if singleTest != "" {
		testNames = []string{singleTest}
	}

	for _, testName := range testNames {
		log.Printf("Running test %s", testName)

		sourceDir := filepath.Join(testsDir, testName)
		must(os.RemoveAll(harnessDir))
		must(os.MkdirAll(harnessDir, 0755))

		targetDir := filepath.Join(harnessDir, testName)
		must(copy.Copy(sourceDir, targetDir))

		cargoPath := filepath.Join(targetDir, "Cargo.toml")
		cargoVars := struct {
			TestName       string
			LavishRevision string
		}{
			TestName:       testName,
			LavishRevision: "ced097658a9246bfc2d7d68f03e97ce0ca98c4d4",
		}

		executeTemplate("Cargo.toml", cargoPath, cargoVars)

		srcPath := filepath.Join(targetDir, "src")
		must(filepath.Walk(srcPath, func (path string, info os.FileInfo, err error) error {
			must(err)

			if filepath.Base(path) == "lavish-rules" {
				workspacePath := filepath.Dir(path)
				log.Printf("Compiling lavish workspace %q", workspacePath)
				must(sh.RunV("../target/debug/lavish", "build", workspacePath))
			}

			return nil
		}))

		must(sh.RunV("cargo", "test", "--manifest-path", cargoPath))
	}
	return nil
}

func executeTemplate(tmplName string, outPath string, data interface{}) {
	tmplPath := filepath.Join("templates", tmplName+".template")

	tmpl, err := template.ParseFiles(tmplPath)
	must(err)

	must(os.MkdirAll(filepath.Dir(outPath), 0755))
	f, err := os.Create(outPath)
	must(err)

	defer f.Close()

	must(tmpl.Execute(f, data))
}

func must(err error) {
	if err != nil {
		panic(fmt.Sprintf("%+v", err))
	}
}
