#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail
if [[ "${TRACE-0}" == "1" ]]; then
	set -o xtrace
fi

help() {
	echo "Usage: ./cli.sh <action>"
	echo "Actions:"
	echo "  -h, --help: Display this help message"
	echo ""
	echo "  lint: Run lint"
	echo ""
	echo "  setup: Setup the project development environment"
	echo "  teardown: Teardown the project development environment"
	echo "  db-console: Open the database console"
	echo "  serve: Run the api server"
	echo "  test: Run tests"
	echo "  coverage: Run tests with coverage"
}

cleanup_profraw_files() {
	find . -type f -name "*.profraw" -delete
}

cd "$(dirname "$0")"/../

tarpaulin_args="--workspace --include-tests --skip-clean --out html --engine llvm -- --show-output --test-threads 1"
tarpaulin_xml_args="--workspace --include-tests --skip-clean --out xml --engine llvm -- --show-output --test-threads 1"

main() {
	action=${1-}

	# shift if length of arguments is greater than 0
	[[ $# -gt 0 ]] && shift 1

	case $action in
	"" | "-h" | "--help")
		help
		exit
		;;
	"lint")
		echo ">>> Running clippy"
		pre-commit run --all-files
		cleanup_profraw_files
		;;
	"setup")
		if [[ -z "${CI-}" ]]; then
			echo ">>> Setting up the project development environment"
			docker compose up -d --wait

			url=postgres://postgres:example@${POSTGRES_HOST-localhost}:${POSTGRES_PORT-5432}/${POSTGRES_DB-bearmark}
			echo "use flake
export BM_DATABASES='{main={url=\"$url\"}}'" >.envrc
			echo ">>> Setting up database"
			DATABASE_URL=$url ./scripts/bin/diesel migration run
		else
			echo ">>> Skip setting up the project development environment"
			echo ">>> Setting up database"
			DATABASE_URL=$url diesel migration run
		fi
		echo ">>> Done"
		;;
	"teardown")
		echo ">>> Tearing down the project development environment"
		docker compose down
		echo ">>> Done"
		;;
	"db-console")
		docker compose exec db psql -Upostgres ${POSTGRES_DB-bearmark}
		;;
	"serve")
		echo ">>> Running the api server"
		cargo run --package bearmark-api --bin serve
		;;
	"test")
		echo ">>> Running tests"
		cargo test --workspace -- --show-output --test-threads 1 "$@"
		cleanup_profraw_files
		;;
	"coverage")
		echo ">>> Running tests with coverage"
		cargo tarpaulin $tarpaulin_args "$@"
		echo "open file ./tarpaulin-report.html to see coverage report"
		cleanup_profraw_files
		;;
	"coverage-xml")
		echo ">>> Running tests with coverage"
		cargo tarpaulin $tarpaulin_xml_args "$@"
		cleanup_profraw_files
		;;
	*)
		echo "Error: Unknown action '$action'"
		help
		exit
		;;
	esac
}

main "$@"
