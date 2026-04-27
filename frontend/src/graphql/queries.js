import { gql } from '@apollo/client';

export const GET_PROJECTS = gql`
  query GetProjects($status: String, $creator: String, $limit: Int, $offset: Int) {
    projects(status: $status, creator: $creator, limit: $limit, offset: $offset) {
      projectId
      creator
      status
      goal
      primaryToken
      createdLedger
    }
  }
`;

export const ACTIVITY_SUBSCRIPTION = gql`
  subscription OnActivityFeed {
    activityFeed {
      id
      eventType
      projectId
      actor
      amount
      ledger
      timestamp
    }
  }
`;
