_basher___refract() {
	local cur prev opts
	COMPREPLY=()
	cur="${COMP_WORDS[COMP_CWORD]}"
	prev="${COMP_WORDS[COMP_CWORD-1]}"
	opts=()
	if [[ ! " ${COMP_LINE} " =~ " -e " ]] && [[ ! " ${COMP_LINE} " =~ " --exit-auto " ]]; then
		opts+=("-e")
		opts+=("--exit-auto")
	fi
	if [[ ! " ${COMP_LINE} " =~ " -h " ]] && [[ ! " ${COMP_LINE} " =~ " --help " ]]; then
		opts+=("-h")
		opts+=("--help")
	fi
	[[ " ${COMP_LINE} " =~ " --no-avif " ]] || opts+=("--no-avif")
	[[ " ${COMP_LINE} " =~ " --no-jxl " ]] || opts+=("--no-jxl")
	[[ " ${COMP_LINE} " =~ " --no-lossless " ]] || opts+=("--no-lossless")
	[[ " ${COMP_LINE} " =~ " --no-lossy " ]] || opts+=("--no-lossy")
	[[ " ${COMP_LINE} " =~ " --no-webp " ]] || opts+=("--no-webp")
	[[ " ${COMP_LINE} " =~ " --no-ycbcr " ]] || opts+=("--no-ycbcr")
	if [[ ! " ${COMP_LINE} " =~ " -s " ]] && [[ ! " ${COMP_LINE} " =~ " --save-auto " ]]; then
		opts+=("-s")
		opts+=("--save-auto")
	fi
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
		--list|-l)
			if [ -z "$( declare -f _filedir )" ]; then
				COMPREPLY=( $( compgen -f "${cur}" ) )
			else
				COMPREPLY=( $( _filedir ) )
			fi
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
