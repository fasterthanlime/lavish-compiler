package main

import (
	"fmt"
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
	fmt.Println("Building lavish compiler")
	must(sh.RunV("cargo", "build", "--manifest-path", "../Cargo.toml"))

	fmt.Println("Running codegen tests...")
	tests := []string{"double"}
	for _, testName := range tests {
		fmt.Printf("Running test %s\n", testName)

		harnessDir := filepath.Join("tmp", "harness")
		sourceDir := filepath.Join("codegen-tests", testName)
		// must(os.RemoveAll(harnessDir))
		must(os.MkdirAll(harnessDir, 0755))

		must(sh.RunV("echo", "Hello world"))

		targetDir := filepath.Join(harnessDir, testName)
		must(copy.Copy(sourceDir, targetDir))

		cargoPath := filepath.Join(targetDir, "Cargo.toml")
		cargoVars := struct {
			TestName       string
			LavishRevision string
		}{
			TestName:       testName,
			LavishRevision: "f89d94bf519bcf91ea31912277640d41e1e6013b",
		}

		executeTemplate("Cargo.toml", cargoPath, cargoVars)

		workspacePath := filepath.Join(targetDir, "src", "services")
		must(sh.RunV("../target/debug/lavish", "build", workspacePath))
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
