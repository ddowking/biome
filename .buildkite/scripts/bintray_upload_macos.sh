#!/bin/bash

set -euo pipefail

# TODO: bintray user = chef-releng-ops!

# We need to upload (but not publish) artifacts to Bintray right now.
channel=$(buildkite-agent meta-data get "release-channel")

# TODO (CM): extract set_hab_binary function to a common library and
# use it here

echo "--- :habicat: Installing core/hab-bintray-publish from '${channel}' channel"
sudo hab pkg install \
     --channel="${channel}" \
     core/hab-bintray-publish

echo "--- :buildkite: Retrieving macOS core/hab artifact"
hab_artifact=$(buildkite-agent meta-data get "hab-artifact-macos")
buildkite-agent artifact download "${hab_artifact}" .

echo "--- :habicat: Uploading macOS core/hab to Bintray"
# We upload to the stable channel, but we don't *publish* until
# later.
#
# -s = skip publishing
# -r = the repository to upload to

# TODO (CM): why do we need the HAB_BLDR_CHANNEL here?
sudo HAB_BLDR_CHANNEL="${channel}" \
     BINTRAY_USER="${BINTRAY_USER}" \
     BINTRAY_KEY="${BINTRAY_KEY}" \
     BINTRAY_PASSPHRASE="${BINTRAY_PASSPHRASE}" \
     hab pkg exec core/hab-bintray-publish \
         publish-hab \
         -s \
         -r stable \
         "${hab_artifact}"

source results/last_build.env
shasum=$(awk '{print $1}' "results/${pkg_artifact:?}.sha256sum")
cat << EOF | buildkite-agent annotate --style=success --context=bintray-hab-macos
<h3>Habitat Bintray Binary (${pkg_target:?})</h3>
Artifact: <code>${pkg_artifact}</code>
<br/>
SHA256: <code>${shasum}</code>
EOF
