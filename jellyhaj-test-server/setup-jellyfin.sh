#!/usr/bin/env bash

set -e
set -u

curl --fail-with-body -X 'POST' 'http://localhost:@port@/Startup/User' -H 'accept: */*' -H 'Content-Type: application/json' \
  -d '{
  "Name": "jellyfin",
  "Password": "jellyfin"
}'

curl --fail-with-body -X 'POST' 'http://localhost:@port@/Startup/Complete' -H 'accept: */*' -d ''
