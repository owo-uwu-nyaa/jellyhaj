#!/usr/bin/env bash

set -e
set -u
# wait until the server has actiuallly started
# jellyfin can't notify systemd it is actually ready
sleep 10
echo sending general configuration
curl --fail-with-body -X 'POST' 'http://localhost:@port@/Startup/Configuration' -H 'accept: */*' -H 'Content-Type: application/json' \
    -H 'authorization: MediaBrowser Client="curl",Device="curl",DeviceId="42",Version="1"' \
    -d '{
    "MetadataCountryCode": "US",
    "PreferredMetadataLanguage": "en",
    "ServerName": "jellyhaj-test-server",
    "UICulture": "en-US"
}'
echo sending network settings

echo sending user information
curl --fail-with-body -X 'POST' 'http://localhost:@port@/Startup/User' -H 'accept: */*' -H 'Content-Type: application/json' \
    -H 'authorization: MediaBrowser Client="curl",Device="curl",DeviceId="42",Version="1"' \
    -d '{
  "Name": "jellyfin",
  "Password": "jellyfin"
}'
echo setting remote access
curl -X 'POST' 'http://localhost:@port@/Startup/RemoteAccess' -H 'accept: */*' -H 'Content-Type: application/json' \
    -H 'authorization: MediaBrowser Client="curl",Device="curl",DeviceId="42",Version="1"' \
    -d '{"EnableRemoteAccess": false}'
echo completing setup
curl --fail-with-body -X 'POST' 'http://localhost:@port@/Startup/Complete' -H 'accept: */*' \
    -H 'authorization: MediaBrowser Client="curl",Device="curl",DeviceId="42",Version="1"' \
    -d ''

touch var/lib/jellyfin/.setup-complete
