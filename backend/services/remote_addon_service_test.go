package services

import (
	"ClassicAddonManager/backend/shared"
	"errors"
	"testing"
)

func newResolution(deps ...shared.DependencyInfo) shared.DependencyResolutionResult {
	return shared.DependencyResolutionResult{
		Dependencies: deps,
		Errors:       []string{},
	}
}

func dep(name, alias string, installed bool, depth int) shared.DependencyInfo {
	return shared.DependencyInfo{
		Manifest: shared.AddonManifest{
			Name:  name,
			Alias: alias,
		},
		IsInstalled: installed,
		Depth:       depth,
	}
}

func TestApplyDependenciesThenParent_DepsBeforeParent(t *testing.T) {
	var order []string
	installDep := func(m shared.AddonManifest, v string) (bool, error) {
		order = append(order, "install:"+m.Name)
		return true, nil
	}
	parent := func() (bool, error) {
		order = append(order, "parent")
		return true, nil
	}
	sort := func() error {
		order = append(order, "sort")
		return nil
	}

	res := newResolution(
		dep("dep-a", "", false, 0),
		dep("dep-b", "", false, 0),
	)
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	if !got.Success {
		t.Fatalf("expected success, got %+v", got)
	}
	if len(order) != 4 || order[0] != "install:dep-a" || order[1] != "install:dep-b" || order[2] != "parent" || order[3] != "sort" {
		t.Fatalf("expected install,install,parent,sort order, got %v", order)
	}
}

func TestApplyDependenciesThenParent_FailedDepAbortsParentAndSort(t *testing.T) {
	parentCalled := false
	sortCalled := false
	installDep := func(m shared.AddonManifest, v string) (bool, error) {
		if m.Name == "dep-a" {
			return false, errors.New("download failed")
		}
		return true, nil
	}
	parent := func() (bool, error) {
		parentCalled = true
		return true, nil
	}
	sort := func() error {
		sortCalled = true
		return nil
	}

	res := newResolution(dep("dep-a", "", false, 0))
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	if parentCalled || sortCalled {
		t.Fatalf("parent/sort must not run on dep failure")
	}
	if got.Success {
		t.Fatalf("expected success false on dep failure")
	}
	if got.MainAddon.Error != "dependency installation failed" {
		t.Fatalf("expected MainAddon.Error 'dependency installation failed', got %q", got.MainAddon.Error)
	}
	if len(got.Dependencies) != 1 || got.Dependencies[0].Name != "dep-a" || got.Dependencies[0].Success {
		t.Fatalf("expected one failed dep row, got %+v", got.Dependencies)
	}
	if got.Dependencies[0].Error != "download failed" {
		t.Fatalf("expected dep error 'download failed', got %q", got.Dependencies[0].Error)
	}
}

func TestRemoteAddonService_InstallAddonFalseNilAbortsParentAndSort(t *testing.T) {
	orig := installAddon
	t.Cleanup(func() { installAddon = orig })
	installAddon = func(shared.AddonManifest, string) (bool, error) {
		return false, nil
	}

	parentCalled := false
	sortCalled := false
	s := &RemoteAddonService{}
	got := applyDependenciesThenParent(
		shared.AddonManifest{Name: "main"},
		newResolution(dep("dep-a", "", false, 0)),
		s.InstallAddon,
		func() (bool, error) {
			parentCalled = true
			return true, nil
		},
		func() error {
			sortCalled = true
			return nil
		},
	)

	if parentCalled || sortCalled {
		t.Fatalf("parent/sort must not run when service InstallAddon returns false,nil")
	}
	if got.Success {
		t.Fatalf("expected success false, got %+v", got)
	}
	if len(got.Dependencies) != 1 || got.Dependencies[0].Success {
		t.Fatalf("expected one failed dep row, got %+v", got.Dependencies)
	}
	if got.Dependencies[0].Error != "installation failed" {
		t.Fatalf("expected fallback 'installation failed', got %q", got.Dependencies[0].Error)
	}
	if got.MainAddon.Error != "dependency installation failed" {
		t.Fatalf("expected MainAddon.Error 'dependency installation failed', got %q", got.MainAddon.Error)
	}
}

func TestApplyDependenciesThenParent_FailedDepFalseNoErr(t *testing.T) {
	installDep := func(m shared.AddonManifest, v string) (bool, error) {
		return false, nil
	}
	parent := func() (bool, error) { return true, nil }
	sort := func() error { return nil }

	res := newResolution(dep("dep-a", "", false, 0))
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	if got.Dependencies[0].Error != "installation failed" {
		t.Fatalf("expected fallback 'installation failed', got %q", got.Dependencies[0].Error)
	}
}

func TestApplyDependenciesThenParent_CatalogMissingDoesNotBlockParent(t *testing.T) {
	parentCalled := false
	installDep := func(m shared.AddonManifest, v string) (bool, error) { return true, nil }
	parent := func() (bool, error) {
		parentCalled = true
		return true, nil
	}
	sort := func() error { return nil }

	res := shared.DependencyResolutionResult{
		Dependencies: []shared.DependencyInfo{dep("dep-a", "", false, 0)},
		Errors:       []string{"Addon ghost not found in repository manifests"},
	}
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	if !parentCalled {
		t.Fatalf("parent must run despite catalog-missing warning")
	}
	if !got.Success {
		t.Fatalf("expected success true")
	}
	if len(got.DependencyWarnings) != 1 || got.DependencyWarnings[0] != "Addon ghost not found in repository manifests" {
		t.Fatalf("expected catalog-missing warning preserved, got %v", got.DependencyWarnings)
	}
}

func TestApplyDependenciesThenParent_SortOnlyAfterSuccessfulParent(t *testing.T) {
	sortCalled := false
	installDep := func(m shared.AddonManifest, v string) (bool, error) { return true, nil }
	parent := func() (bool, error) { return false, nil }
	sort := func() error {
		sortCalled = true
		return nil
	}

	res := newResolution(dep("dep-a", "", false, 0))
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	if sortCalled {
		t.Fatalf("sort must not run on parent failure")
	}
	if got.Success {
		t.Fatalf("expected success false on parent failure")
	}
	if got.MainAddon.Success {
		t.Fatalf("expected MainAddon.Success false")
	}
	if got.MainAddon.Error != "installation failed" {
		t.Fatalf("expected fallback 'installation failed', got %q", got.MainAddon.Error)
	}
}

func TestApplyDependenciesThenParent_ParentErrPropagatedToMain(t *testing.T) {
	installDep := func(m shared.AddonManifest, v string) (bool, error) { return true, nil }
	parent := func() (bool, error) { return false, errors.New("release fetch failed") }
	sort := func() error { return nil }

	res := newResolution()
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	if got.MainAddon.Error != "release fetch failed" {
		t.Fatalf("expected parent error on MainAddon, got %q", got.MainAddon.Error)
	}
	if got.Success {
		t.Fatalf("expected success false")
	}
}

func TestApplyDependenciesThenParent_SortErrorKeepsSuccessAndWarns(t *testing.T) {
	installDep := func(m shared.AddonManifest, v string) (bool, error) { return true, nil }
	parent := func() (bool, error) { return true, nil }
	sort := func() error { return errors.New("circular dependency detected: a -> b -> a") }

	res := newResolution(dep("dep-a", "", false, 0))
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	if !got.Success {
		t.Fatalf("sort error must keep overall success true")
	}
	if !got.MainAddon.Success {
		t.Fatalf("parent succeeded, MainAddon.Success must be true")
	}
	sortErr := "circular dependency detected: a -> b -> a"
	if got.MainAddon.Error != sortErr {
		t.Fatalf("expected MainAddon.Error = sort error, got %q", got.MainAddon.Error)
	}
	if len(got.DependencyWarnings) != 1 || got.DependencyWarnings[0] != sortErr {
		t.Fatalf("expected sort error appended to warnings, got %v", got.DependencyWarnings)
	}
}

func TestApplyDependenciesThenParent_AlreadyInstalledSkipped(t *testing.T) {
	installCalled := false
	installDep := func(m shared.AddonManifest, v string) (bool, error) {
		installCalled = true
		return true, nil
	}
	parent := func() (bool, error) { return true, nil }
	sort := func() error { return nil }

	res := newResolution(dep("dep-a", "Alias A", true, 0))
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main", Alias: "Main Alias"}, res, installDep, parent, sort)

	if installCalled {
		t.Fatalf("install must not be called for already-installed dep")
	}
	if len(got.Dependencies) != 1 {
		t.Fatalf("expected one dep row")
	}
	row := got.Dependencies[0]
	if !row.Success || !row.Skipped {
		t.Fatalf("expected skipped row Success+Skipped, got %+v", row)
	}
	if row.Name != "dep-a" || row.Alias != "Alias A" {
		t.Fatalf("expected name/alias preserved, got %+v", row)
	}
	if !got.Success {
		t.Fatalf("expected overall success")
	}
	if got.MainAddon.Name != "main" || got.MainAddon.Alias != "Main Alias" {
		t.Fatalf("expected MainAddon name/alias from ad, got %+v", got.MainAddon)
	}
}

func TestApplyDependenciesThenParent_AliasFallbackToName(t *testing.T) {
	installDep := func(m shared.AddonManifest, v string) (bool, error) { return true, nil }
	parent := func() (bool, error) { return true, nil }
	sort := func() error { return nil }

	res := newResolution(dep("dep-a", "", false, 0))
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	if got.Dependencies[0].Alias != "dep-a" {
		t.Fatalf("expected alias fallback to name, got %q", got.Dependencies[0].Alias)
	}
}

func TestApplyDependenciesThenParent_SuccessfulDep(t *testing.T) {
	installDep := func(m shared.AddonManifest, v string) (bool, error) { return true, nil }
	parent := func() (bool, error) { return true, nil }
	sort := func() error { return nil }

	res := newResolution(dep("dep-a", "A", false, 0))
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	row := got.Dependencies[0]
	if !row.Success || row.Skipped {
		t.Fatalf("expected successful non-skipped row, got %+v", row)
	}
	if row.Alias != "A" {
		t.Fatalf("expected alias A, got %q", row.Alias)
	}
}

func TestApplyDependenciesThenParent_FailedDepAfterEarlierSuccess(t *testing.T) {
	installDep := func(m shared.AddonManifest, v string) (bool, error) {
		if m.Name == "dep-b" {
			return false, errors.New("boom")
		}
		return true, nil
	}
	parent := func() (bool, error) { return true, nil }
	sort := func() error { return nil }

	res := newResolution(
		dep("dep-a", "", false, 0),
		dep("dep-b", "", false, 0),
	)
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	if len(got.Dependencies) != 2 {
		t.Fatalf("expected two dep rows (earlier success retained), got %d", len(got.Dependencies))
	}
	if !got.Dependencies[0].Success {
		t.Fatalf("earlier successful row must be retained, got %+v", got.Dependencies[0])
	}
	if got.Dependencies[1].Success || got.Dependencies[1].Error != "boom" {
		t.Fatalf("expected failed second row, got %+v", got.Dependencies[1])
	}
	if got.Success {
		t.Fatalf("expected overall success false")
	}
	if got.MainAddon.Error != "dependency installation failed" {
		t.Fatalf("expected MainAddon.Error, got %q", got.MainAddon.Error)
	}
}

func TestApplyDependenciesThenParent_NoDepsParentSuccess(t *testing.T) {
	installDep := func(m shared.AddonManifest, v string) (bool, error) { return true, nil }
	parent := func() (bool, error) { return true, nil }
	sort := func() error { return nil }

	res := newResolution()
	got := applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	if !got.Success || !got.MainAddon.Success {
		t.Fatalf("expected success, got %+v", got)
	}
	if len(got.Dependencies) != 0 {
		t.Fatalf("expected no dep rows, got %d", len(got.Dependencies))
	}
}

func TestApplyDependenciesThenParent_InstallDepVersionIsLatest(t *testing.T) {
	var versions []string
	installDep := func(m shared.AddonManifest, v string) (bool, error) {
		versions = append(versions, v)
		return true, nil
	}
	parent := func() (bool, error) { return true, nil }
	sort := func() error { return nil }

	res := newResolution(dep("dep-a", "", false, 0))
	applyDependenciesThenParent(shared.AddonManifest{Name: "main"}, res, installDep, parent, sort)

	for _, v := range versions {
		if v != "latest" {
			t.Fatalf("expected deps installed at 'latest', got %q", v)
		}
	}
}
