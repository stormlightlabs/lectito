#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
DIM='\033[2m'
RESET='\033[0m'
BOLD='\033[1m'

usage() {
	echo -e "${WHITE}Usage:${RESET} $0 [${CYAN}-o${RESET}|${CYAN}--output-dir${RESET} ${DIM}<dir>${RESET}] [${CYAN}-i${RESET}|${CYAN}--input${RESET} ${DIM}<file>${RESET}]"
	echo -e ""
	echo -e "  ${CYAN}-o, --output-dir${RESET}  Directory to write markdown files (default: ${DIM}./out${RESET})"
	echo -e "  ${CYAN}-i, --input${RESET}       URL list file (default: ${DIM}$(dirname "$0")/example.txt${RESET})"
	echo -e "  ${CYAN}-h, --help${RESET}        Show this help"
	exit 0
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="./out"
INPUT_FILE="$SCRIPT_DIR/example.txt"

while [[ $# -gt 0 ]]; do
	case "$1" in
	-o | --output-dir)
		OUTPUT_DIR="$2"
		shift 2
		;;
	-i | --input)
		INPUT_FILE="$2"
		shift 2
		;;
	-h | --help) usage ;;
	*)
		echo -e "${RED}Unknown option:${RESET} $1"
		usage
		;;
	esac
done

if [[ ! -f "$INPUT_FILE" ]]; then
	echo -e "${RED}Error:${RESET} Input file not found: ${WHITE}$INPUT_FILE${RESET}"
	exit 1
fi

mkdir -p "$OUTPUT_DIR"

# Derive a slug from a URL: {subdomain}_{sld}_{path[0]}_{path[n]}
# e.g. https://old.reddit.com/r/rust/comments/abc/ -> old_reddit_r_rust_comments_abc
url_to_filename() {
	local url="$1"
	local rest="${url#*://}"
	local host path

	if [[ "$rest" == */* ]]; then
		host="${rest%%/*}"
		path="${rest#*/}"
	else
		host="$rest"
		path=""
	fi

	host="${host%%:*}"

	IFS='.' read -ra hparts <<<"$host"
	local n=${#hparts[@]}

	local subdomain sld
	if ((n >= 3)); then
		sld="${hparts[$((n - 2))]}"
		local sub_parts=("${hparts[@]:0:$((n - 2))}")
		subdomain=$(
			IFS='.'
			echo "${sub_parts[*]}"
		)
	elif ((n == 2)); then
		sld="${hparts[0]}"
		subdomain=""
	else
		sld="${hparts[0]}"
		subdomain=""
	fi

	path="${path%%\?*}"
	path="${path%%#*}"

	IFS='/' read -ra segs <<<"$path"
	local path_parts=()
	for seg in "${segs[@]}"; do
		[[ -z "$seg" ]] && continue
		seg="${seg%.*}"
		[[ -z "$seg" ]] && continue
		path_parts+=("$seg")
	done

	local slug
	if [[ -n "$subdomain" ]]; then
		slug="${subdomain}_${sld}"
	else
		slug="$sld"
	fi

	for seg in "${path_parts[@]}"; do
		slug="${slug}_${seg}"
	done

	slug=$(echo "$slug" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9_-]/_/g' | sed 's/_\+/_/g' | sed 's/^_//;s/_$//')

	echo "$slug"
}

URLS=()
while IFS= read -r line; do
	[[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
	URLS+=("$line")
done <"$INPUT_FILE"

TOTAL=${#URLS[@]}
PASS=0
FAIL=0

echo -e ""
echo -e "${BOLD}${WHITE}lectito batch extractor${RESET}"
echo -e "${DIM}Input:  ${RESET}${WHITE}$INPUT_FILE${RESET}"
echo -e "${DIM}Output: ${RESET}${WHITE}$OUTPUT_DIR${RESET}"
echo -e "${DIM}URLs:   ${RESET}${WHITE}$TOTAL${RESET}"
echo -e ""

for i in "${!URLS[@]}"; do
	URL="${URLS[$i]}"
	IDX=$((i + 1))
	FILENAME="$(url_to_filename "$URL").md"
	OUT_PATH="$OUTPUT_DIR/$FILENAME"

	echo -e "${DIM}[$IDX/$TOTAL]${RESET} ${CYAN}${URL}${RESET}"
	echo -e "        ${DIM}→ $OUT_PATH${RESET}"

	if cargo run -q -p lectito-cli -- "$URL" --output "$OUT_PATH" 2>/dev/null; then
		echo -e "        ${GREEN}✓ done${RESET}"
		PASS=$((PASS + 1))
	else
		echo -e "        ${RED}✗ failed${RESET}"
		FAIL=$((FAIL + 1))
	fi
	echo ""
done

echo -e "${BOLD}Summary${RESET}"
echo -e "  ${GREEN}Passed:${RESET} $PASS"
if [[ $FAIL -gt 0 ]]; then
	echo -e "  ${RED}Failed:${RESET} $FAIL"
fi
echo ""
