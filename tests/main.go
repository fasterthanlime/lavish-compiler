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

	fmt.Println("Building lavish compiler")
	must(sh.RunV("cargo", "build", "--manifest-path", "../Cargo.toml"))

	fmt.Println("Running codegen tests...")

	tmpDir := filepath.Join(cwd, "tmp")

	cargoTargetDir := filepath.Join(tmpDir, "target")
	os.Setenv("CARGO_TARGET_DIR", cargoTargetDir)

	harnessDir := filepath.Join(tmpDir, "harness")

	for _, testName := range testNames {
		fmt.Printf("Running test %s\n", testName)

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
			LavishRevision: "51aa2bc653454931253c6a396dc160652e458566",
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
