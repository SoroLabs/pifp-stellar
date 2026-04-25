#!/bin/bash

# Test script for the new donors endpoint
# Usage: ./test_donors_endpoint.sh [base_url] [project_id]

BASE_URL=${1:-"http://localhost:8080"}
PROJECT_ID=${2:-"test_project"}

echo "Testing Donors API Endpoint"
echo "============================"
echo "Base URL: $BASE_URL"
echo "Project ID: $PROJECT_ID"
echo ""

# Test 1: Get first page of donors
echo "Test 1: Get first page of donors (default limit=20, offset=0)"
curl -s "$BASE_URL/projects/$PROJECT_ID/donors" | jq '.'
echo ""

# Test 2: Get donors with custom pagination
echo "Test 2: Get donors with limit=5, offset=0"
curl -s "$BASE_URL/projects/$PROJECT_ID/donors?limit=5&offset=0" | jq '.'
echo ""

# Test 3: Get second page
echo "Test 3: Get second page with limit=5, offset=5"
curl -s "$BASE_URL/projects/$PROJECT_ID/donors?limit=5&offset=5" | jq '.'
echo ""

# Test 4: Test with large limit (should be clamped to 100)
echo "Test 4: Test with large limit=1000 (should be clamped to 100)"
curl -s "$BASE_URL/projects/$PROJECT_ID/donors?limit=1000" | jq '.donors | length'
echo ""

# Test 5: Test with non-existent project
echo "Test 5: Test with non-existent project"
curl -s "$BASE_URL/projects/non_existent_project/donors" | jq '.'
echo ""

echo "Testing complete!"