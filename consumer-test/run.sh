#!/usr/bin/env bash
# Consumer test: installs this binding into an empty project through its
# package manager, as a user does, and runs the README quick start
# (consumer-test/quickstart.<ext>) and the consumer contract.
#
#   run.sh local <dist-dir> <version>   from the artifacts package.yml built
#   run.sh registry <version>           from the public registry, after publish
#
# The contract every binding checks, and exits non-zero if any part fails:
#   - in memory: a transact, then a query that returns the transacted value;
#   - file-backed: transact into a new .graph file, close, reopen, query;
#   - the error type: an invalid command raises the binding's error type, and
#     its message carries the error code ("[PRS-001] ...");
#   - the installed version is <version>.
#
# TODO(binding): fill in the three TODO blocks below. See minigraf-python,
# minigraf-node or minigraf-c for complete examples.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# wait_for <seconds> <command...>: run the command every 30 s until it succeeds.
# Registries can take minutes to serve a new version.
wait_for() {
  local limit="$1"; shift
  local start=$SECONDS
  until "$@"; do
    if (( SECONDS - start >= limit )); then
      echo "::error::gave up after ${limit}s waiting for: $*"
      return 1
    fi
    sleep 30
  done
}

# Resolve the dist directory before leaving the caller's directory.
if [ "${1:-}" = local ]; then DIST="$(cd "$2" && pwd)"; fi

APP="$(mktemp -d)"
trap 'rm -rf "$APP"' EXIT
cd "$APP"

case "${1:-}" in
  local)
    VERSION="$3"
    # TODO(binding): install the artifacts in $DIST into a fresh project in
    # $APP with your package manager (pip install --no-index --find-links,
    # npm install <tgz>, a Gradle mavenLocal-style repo, ...). The template's
    # placeholder only checks what package.yml staged.
    for f in libminigraf_ffi.so LICENSE-MIT LICENSE-APACHE; do
      [ -f "$DIST/$f" ] || { echo "::error::$DIST has no $f"; exit 1; }
    done
    [ "$(cat "$DIST/VERSION")" = "$VERSION" ] || { echo "::error::dist is not version $VERSION"; exit 1; }
    echo "template placeholder: dist-native holds version $VERSION"
    exit 0
    ;;
  registry)
    VERSION="$2"
    # TODO(binding): a check that the registry serves $VERSION, for example
    #   published() { [ "$(npm view "<package>@$VERSION" version 2>/dev/null)" = "$VERSION" ]; }
    #   wait_for 600 published
    # then install <package>==$VERSION into $APP.
    echo "::error::registry mode is not filled in for this binding"
    exit 1
    ;;
  *)
    echo "usage: $0 local <dist-dir> <version> | registry <version>" >&2
    exit 2
    ;;
esac

# TODO(binding): check the installed version, copy "$HERE"/quickstart.<ext>
# and the contract test into $APP, and run them.
