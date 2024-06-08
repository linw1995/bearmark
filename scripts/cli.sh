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
	echo "  setup: Setup the project development environment"
	echo "  serve: Run the api server"
	echo "  test: Run tests"
	echo "  install-diesel: Install diesel_cli"
}

cd "$(dirname "$0")"/../

main() {
	# Set the library path for the diesel_cli to find the libpq library
	# This is required for the diesel_cli to work
	# Use `brew install libpq` to install the libpq library
	# Use `brew info libpq` to get the library path if below path is not correct
	export LIBRARY_PATH=${LIBRARY_PATH-}:/opt/homebrew/opt/libpq/lib/

	action=${1-}

	case $action in
	"" | "-h" | "--help")
		help
		exit
		;;
	"setup")
		echo ">>> Setting up the project development environment"
		docker compose up -d
		echo ">>> Setting up database"
		echo "DATABASE_URL=postgres://postgres:example@localhost/bmm" >.env
		diesel migration run
		echo ">>> Done"
		;;
	"serve")
		echo ">>> Running the api server"
		cargo run
		;;
	"test")
		echo ">>> Running tests"
		cargo test -- --show-output
		;;
	"install-diesel")
		echo ">>> Installing diesel_cli"
		cargo install diesel_cli --no-default-features --features postgres
		;;
	*)
		echo "Error: Unknown action '$action'"
		help
		exit
		;;
	esac
}

main "$@"
