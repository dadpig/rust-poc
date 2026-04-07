#!/usr/bin/env bash
set -euo pipefail

BASE_URL="http://localhost:3000"
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

step() { echo -e "\n${CYAN}==> $1${NC}"; }
ok()   { echo -e "${GREEN}$1${NC}"; }
warn() { echo -e "${YELLOW}$1${NC}"; }
fail() { echo -e "${RED}$1${NC}"; }

step "GET /items (empty list)"
curl -s -w "\nHTTP %{http_code}" "$BASE_URL/items" | tail -1 | grep -q "HTTP 200" && ok "200 OK" || fail "FAILED"
curl -s "$BASE_URL/items" | python3 -m json.tool

step "POST /items (create first item)"
RESPONSE=$(curl -s -X POST "$BASE_URL/items" \
  -H "Content-Type: application/json" \
  -d '{"name":"sword","description":"a sharp blade"}')
echo "$RESPONSE" | python3 -m json.tool
ID=$(echo "$RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])")
ok "Created item ID: $ID"

step "POST /items (create second item)"
curl -s -X POST "$BASE_URL/items" \
  -H "Content-Type: application/json" \
  -d '{"name":"shield","description":"a round shield"}' | python3 -m json.tool

step "GET /items (list all)"
curl -s "$BASE_URL/items" | python3 -m json.tool

step "GET /items/$ID (get by id)"
curl -s "$BASE_URL/items/$ID" | python3 -m json.tool

step "PUT /items/$ID (full update)"
curl -s -X PUT "$BASE_URL/items/$ID" \
  -H "Content-Type: application/json" \
  -d '{"name":"great sword","description":"a very large blade"}' | python3 -m json.tool

step "PATCH /items/$ID (partial update — name only)"
curl -s -X PATCH "$BASE_URL/items/$ID" \
  -H "Content-Type: application/json" \
  -d '{"name":"legendary sword"}' | python3 -m json.tool

step "GET /items/$ID (verify patch)"
curl -s "$BASE_URL/items/$ID" | python3 -m json.tool

step "DELETE /items/$ID"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$BASE_URL/items/$ID")
[ "$STATUS" = "204" ] && ok "204 No Content" || fail "Expected 204, got $STATUS"

step "GET /items/$ID (expect 404 after delete)"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/items/$ID")
[ "$STATUS" = "404" ] && ok "404 Not Found (correct)" || fail "Expected 404, got $STATUS"

step "GET /items (final list)"
curl -s "$BASE_URL/items" | python3 -m json.tool

echo -e "\n${GREEN}Done.${NC}"
