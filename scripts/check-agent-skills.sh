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

npx --yes "skills@${skills_cli_version}" add "${skills_source}" \
    --agent codex \
    --list \
    --yes >"${output_file}" 2>&1

grep -F "Found 1 skill" "${output_file}"
grep -E '^[^[:alnum:]]*complai[[:space:]]*$' "${output_file}"

# These guides belong to the installed `complai` binary. Discovering any of
# them here would reintroduce a separately cached, version-drifting copy.
if grep -E '^[^[:alnum:]]*(project-init|doc-ingest|gap-analysis)[[:space:]]*$' "${output_file}"; then
    echo "npx skills exposed an internal workflow instead of only complai" >&2
    exit 1
fi

echo "npx skills discovered only the complai discovery skill from ${skills_source}"
