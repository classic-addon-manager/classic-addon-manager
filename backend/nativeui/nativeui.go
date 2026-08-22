package nativeui

const nativeErrorFlag = "--native-error"

func parseNativeErrorArgs(args []string) (title, message string, ok bool) {
	if len(args) != 3 || args[0] != nativeErrorFlag {
		return "", "", false
	}

	return args[1], args[2], true
}
