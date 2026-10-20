#!/usr/bin/env bash
set -euo pipefail

repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
container_name="jenkins-sdk-lts-${$}"
image="jenkins/jenkins:2.580.1-jdk21"

cleanup() {
    docker rm --force "$container_name" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run --detach --rm --name "$container_name" \
    --publish 127.0.0.1::8080 \
    --env JAVA_OPTS=-Djenkins.install.runSetupWizard=false \
    --mount "type=bind,src=$repo_dir/tests/fixtures/jenkins/init.groovy,dst=/usr/share/jenkins/ref/init.groovy.d/sdk.groovy,readonly" \
    "$image" --prefix=/jenkins >/dev/null

address=$(docker port "$container_name" 8080)
export JENKINS_SDK_TEST_URL="http://$address/jenkins"
ready=false
for ((attempt = 0; attempt < 120; attempt++)); do
    if curl --fail --silent --max-time 2 \
        --user sdk-admin:sdk-integration-password \
        "$JENKINS_SDK_TEST_URL/api/json" >/dev/null; then
        ready=true
        break
    fi
    sleep 1
done
if [[ $ready != true ]]; then
    docker logs "$container_name"
    exit 1
fi

cd "$repo_dir"
if ! cargo nextest run --features blocking-rustls-ring --test core_lts --run-ignored only --no-capture; then
    docker logs --tail 100 "$container_name"
    exit 1
fi
