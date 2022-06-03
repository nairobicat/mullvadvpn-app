#!/usr/bin/env bash
set -eu
shopt -s nullglob
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
# In the CI environment we would like to import trusted public keys from a file, but not in our build environment
import_gpg_keys="false"
# The policy of enforcing lockfiles to be signed was not in place before this commit and as such some of the commits before are not signed
# The whitelisted commit can be set in order to allow github actions to only check changes since origin/master
whitelisted_commit="5d41b8a1d9745fbb3ff81ea6ea2eb8f202ca7ed0"

while [ ! $# -eq 0 ]; do
    case "$1" in
        "--import-gpg-keys")
            import_gpg_keys="true"
            ;;
        "--whitelist")
            whitelisted_commit="$2"
            shift
            ;;
        -*)
            echo "Unknown option \"$1\"
The options are --import-gpg-keys and --whitelist"
            exit 1
            ;;
        *)
            echo "Unknown argument
The options are --import-gpg-keys and --whitelist"
            exit 1
            ;;
    esac
    shift
done

if [[ "$import_gpg_keys" == "true" ]]; then
    GNUPGHOME=$(mktemp -d)
    for key in $SCRIPT_DIR/keys/*; do
        gpg --import --armor $key
    done
fi

unsigned_commits_exist=0
LOCKED_DOWN_FILES=$(cat $SCRIPT_DIR/locked_down_files.txt)
for locked_file in $LOCKED_DOWN_FILES; do
    locked_file_commit_hashes=$(git rev-list --oneline $whitelisted_commit..HEAD $SCRIPT_DIR/../$locked_file | awk '{print $1}')
    for commit in $locked_file_commit_hashes;
    do
        if ! $(git verify-commit $commit 2> /dev/null); then
            echo Commit $commit which changed $locked_file is not signed.
            unsigned_commits_exist=1
        fi
    done
done

exit $unsigned_commits_exist
