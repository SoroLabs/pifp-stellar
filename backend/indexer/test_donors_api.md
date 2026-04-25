# Donor List API Implementation

## Overview
This implementation adds pagination support for the donor list API endpoint to improve performance for projects with thousands of participants.

## New Endpoint

### `GET /projects/:id/donors`

Returns a paginated list of donors for a specific project.

#### Query Parameters
- `limit` (optional): Number of donors to return per page (default: 20, max: 100)
- `offset` (optional): Number of donors to skip (default: 0)

#### Response Format
```json
{
  "project_id": "string",
  "total_donors": 1234,
  "donors": [
    {
      "address": "donor_address_1",
      "total_donated": "5000",
      "donation_count": 3,
      "first_donation_ledger": 12345,
      "last_donation_ledger": 12567,
      "first_donation_timestamp": 1640995200,
      "last_donation_timestamp": 1641081600
    }
  ]
}
```

#### Sorting
Donors are sorted by:
1. Total donated amount (descending) - highest donors first
2. First donation timestamp (ascending) - earlier donors first for ties

## Database Functions Added

1. `get_project_donors(pool, project_id, limit, offset)` - Fetches paginated donor list
2. `get_project_donors_count(pool, project_id)` - Gets total count of unique donors

## Features

- **Pagination**: Supports limit/offset pagination
- **Consistent Sorting**: Deterministic order for reliable pagination
- **Performance**: Uses efficient SQL queries with proper indexing
- **Comprehensive Data**: Includes donation statistics per donor
- **Parallel Queries**: Fetches count and donors simultaneously for better performance

## Example Usage

```bash
# Get first 20 donors
curl "http://localhost:8080/projects/project_123/donors"

# Get next 20 donors
curl "http://localhost:8080/projects/project_123/donors?limit=20&offset=20"

# Get 50 donors starting from position 100
curl "http://localhost:8080/projects/project_123/donors?limit=50&offset=100"
```

## Testing

The implementation includes comprehensive tests:
- Basic donor retrieval and sorting
- Pagination functionality
- Donor count accuracy
- Edge cases (no donors, single donor, etc.)