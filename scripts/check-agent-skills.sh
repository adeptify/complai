#!/bin/sh

set -eu

# Keep the external distribution contract separate from the Rust registry: the
# repository must expose only the lightweight discovery stub to `npx skills`.
repository_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
artifacts_dir="${repository_root}/tmp/agent-skills-check"
output_file="${artifacts_dir}/npx-skills-list.txt"
skills_cli_version=${SKILLS_CLI_VERSION:-1.5.21}
skills_source=${SKILLS_SOURCE:-${repository_root}}

mkdir -p "${artifacts_dir}"

set +e
NO_COLOR=1 FORCE_COLOR=0 npx --yes "skills@${skills_cli_version}" add "${skills_source}" \
    --agent codex \
    --list \
    --yes >"${output_file}" 2>&1
npx_status=$?
set -e

# Always expose the captured output in CI so an upstream CLI change is
# diagnosable without downloading runner artifacts.
cat "${output_file}"
if [ "${npx_status}" -ne 0 ]; then
    exit "${npx_status}"
fi

grep -F "Found 1 skill" "${output_file}"
grep -F "    complai" "${output_file}"

# These guides belong to the installed `complai` binary. Discovering any of
# them here would reintroduce a separately cached, version-drifting copy.
for internal_skill in project-init doc-ingest gap-analysis; do
    if grep -F "    ${internal_skill}" "${output_file}"; then
        echo "npx skills exposed internal workflow ${internal_skill}" >&2
        exit 1
    fi
done

echo "npx skills discovered only the complai discovery skill from ${skills_source}"
