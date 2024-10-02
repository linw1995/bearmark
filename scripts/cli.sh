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
	echo "  lint: Run clippy"
	echo ""
	echo "  setup: Setup the project development environment"
	echo "  teardown: Teardown the project development environment"
	echo "  db-console: Open the database console"
	echo "  serve: Run the api server"
	echo "  test: Run tests"
	echo "  coverage: Run tests with coverage"
	echo ""
	echo "  install-deps: Install all dependencies"
	echo "  install-diesel: Install diesel_cli"
	echo "  install-tarpaulin: Install tarpaulin"
}

cleanup_profraw_files() {
	find . -type f -name "*.profraw" -delete
}

cd "$(dirname "$0")"/../

tarpaulin_args="--workspace --include-tests --skip-clean --out html -- --show-output --test-threads 1"
tarpaulin_xml_args="--workspace --include-tests --skip-clean --out xml -- --show-output --test-threads 1"

export RUST_SRC_PATH="$(rustc --print sysroot)/lib/rustlib/src/rust/src"
export DYLD_LIBRARY_PATH="$(rustc --print sysroot)/lib:${DYLD_LIBRARY_PATH-}"

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
		cargo clippy --all-features "$@"
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
			url=postgres://postgres:example@${POSTGRES_HOST-db}:${POSTGRES_PORT-5432}/${POSTGRES_DB-bearmark}
			echo "export BM_DATABASES='{main={url=\"$url\"}}'" >.envrc
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
		if [[ -z "${CI-}" ]]; then
			./scripts/bin/cargo-tarpaulin $tarpaulin_args "$@"
		else
			cargo tarpaulin $tarpaulin_args "$@"
		fi
		echo "open file ./tarpaulin-report.html to see coverage report"
		cleanup_profraw_files
		;;
	"coverage-xml")
		echo ">>> Running tests with coverage"
		if [[ -z "${CI-}" ]]; then
			./scripts/bin/cargo-tarpaulin $tarpaulin_xml_args "$@"
		else
			cargo tarpaulin $tarpaulin_xml_args "$@"
		fi
		cleanup_profraw_files
		;;
	"install-deps")
		$0 install-diesel
		$0 install-tarpaulin
		;;
	"install-diesel")
		echo ">>> Installing diesel_cli"
		cargo install diesel_cli --no-default-features --features postgres --root ./scripts
		;;
	"install-tarpaulin")
		echo ">>> Installing tarpaulin"
		cargo install cargo-tarpaulin --root ./scripts --git https://github.com/xd009642/tarpaulin.git
		;;
	*)
		echo "Error: Unknown action '$action'"
		help
		exit
		;;
	esac
}

main "$@"
