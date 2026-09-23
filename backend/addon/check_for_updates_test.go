package addon

import "testing"

func TestGenerateUpdatesLuaQuotesEveryField(t *testing.T) {
	tests := []struct {
		name  string
		addon Addon
		want  string
	}{
		{
			name:  "simple name with no alias",
			addon: Addon{Name: "Simple", Version: "1.0"},
			want:  "{\n    [\"Simple\"] = {name=\"Simple\", version=\"1.0\"}, \n}\n",
		},
		{
			name: "quotes slashes and line breaks",
			addon: Addon{
				Name:    "O'Brien \"Beta\"\\Pack\nLine",
				Alias:   "Alias \"quoted\"\\dir\r\nNext",
				Version: "v1\n2\x00",
			},
			want: "{\n    [\"O'Brien \\\"Beta\\\"\\\\Pack\\010Line\"] = {name=\"Alias \\\"quoted\\\"\\\\dir\\013\\010Next\", version=\"v1\\0102\\000\"}, \n}\n",
		},
		{
			name:  "unambiguous decimal escape before digits",
			addon: Addon{Name: "Line\n123", Version: "\t45"},
			want:  "{\n    [\"Line\\010123\"] = {name=\"Line\\010123\", version=\"\\00945\"}, \n}\n",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := string(generateUpdatesLua(map[string]Addon{tt.addon.Name: tt.addon}))
			if got != tt.want {
				t.Errorf("generated Lua = %q, want %q", got, tt.want)
			}
		})
	}
}

