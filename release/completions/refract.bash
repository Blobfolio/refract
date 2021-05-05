_basher___refract() {
	local cur prev opts
	COMPREPLY=()
	cur="${COMP_WORDS[COMP_CWORD]}"
	prev="${COMP_WORDS[COMP_CWORD-1]}"
	opts=()

	if [[ ! " ${COMP_LINE} " =~ " -h " ]] && [[ ! " ${COMP_LINE} " =~ " --help " ]]; then
		opts+=("-h")
		opts+=("--help")
	fi
	[[ " ${COMP_LINE} " =~ " --no-avif " ]] || opts+=("--no-avif")
	[[ " ${COMP_LINE} " =~ " --no-jxl " ]] || opts+=("--no-jxl")
	[[ " ${COMP_LINE} " =~ " --no-webp " ]] || opts+=("--no-webp")
	[[ " ${COMP_LINE} " =~ " --skip-ycbcr " ]] || opts+=("--skip-ycbcr")
	if [[ ! " ${COMP_LINE} " =~ " -V " ]] && [[ ! " ${COMP_LINE} " =~ " --version " ]]; then
		opts+=("-V")
		opts+=("--version")
	fi
	if [[ ! " ${COMP_LINE} " =~ " -l " ]] && [[ ! " ${COMP_LINE} " =~ " --list " ]]; then
		opts+=("-l")
		opts+=("--list")
	fi

	opts=" ${opts[@]} "
	if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]] ; then
		COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
		return 0
	fi

	case "${prev}" in
		-l|--list)
			COMPREPLY=( $( compgen -f "${cur}" ) )
			return 0
			;;
		*)
			COMPREPLY=()
			;;
	esac

	COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
	return 0
}
complete -F _basher___refract -o bashdefault -o default refract
