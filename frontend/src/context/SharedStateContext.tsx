import React, { createContext, useContext, useEffect, useRef, useState, useReducer } from 'react';
import { CrossTabManager } from './CrossTabManager';

interface WalletState {
  isConnected: boolean;
  address: string | null;
  nonce: number;
}

const LOCAL_STORAGE_KEY = 'pifp_wallet_state';

function getInitialState(): WalletState {
  try {
    const cached = localStorage.getItem(LOCAL_STORAGE_KEY);
    if (cached) {
      return JSON.parse(cached);
    }
  } catch (e) {
    console.error('Failed to parse cached wallet state', e);
  }
  return {
    isConnected: false,
    address: null,
    nonce: 0,
  };
}

const initialState: WalletState = getInitialState();

type Action = 
  | { type: 'CONNECT_WALLET'; address: string }
  | { type: 'DISCONNECT_WALLET' }
  | { type: 'INCREMENT_NONCE' }
  | { type: 'SYNC_STATE'; state: WalletState };

function reducer(state: WalletState, action: Action): WalletState {
  switch (action.type) {
    case 'CONNECT_WALLET':
      return { ...state, isConnected: true, address: action.address };
    case 'DISCONNECT_WALLET':
      return { ...state, isConnected: false, address: null };
    case 'INCREMENT_NONCE':
      return { ...state, nonce: state.nonce + 1 };
    case 'SYNC_STATE':
      return { ...action.state };
    default:
      return state;
  }
}

interface SharedStateContextValue {
  state: WalletState;
  isLeader: boolean;
  dispatch: (action: Action) => void;
}

const SharedStateContext = createContext<SharedStateContextValue | undefined>(undefined);

export function SharedStateProvider({ children }: { children: React.ReactNode }) {
  const [state, dispatchLocal] = useReducer(reducer, initialState);
  const [isLeader, setIsLeader] = useState(false);
  const managerRef = useRef<CrossTabManager | null>(null);

  // We need a ref for the latest state to provide it synchronously inside CrossTabManager callbacks
  const stateRef = useRef(state);
  stateRef.current = state;

  useEffect(() => {
    managerRef.current = new CrossTabManager(
      'pifp-stellar-shared-state',
      () => stateRef.current,
      (syncedState: WalletState) => {
        dispatchLocal({ type: 'SYNC_STATE', state: syncedState });
      },
      (action: Action) => {
        dispatchLocal(action);
      },
      (leaderStatus: boolean) => {
        setIsLeader(leaderStatus);
      }
    );

    // Mock WebSocket connection handled ONLY by the leader
    const wsInterval = setInterval(() => {
      if (managerRef.current?.isLeader && stateRef.current.isConnected) {
        // Simulate network activity incrementing the nonce
        dispatchLocal({ type: 'INCREMENT_NONCE' });
      }
    }, 5000);

    return () => {
      clearInterval(wsInterval);
      // Window unload cleanup is handled inside the manager
    };
  }, []);

  // When state changes and we are leader, broadcast it to followers
  // Also save to localStorage for instantaneous hydration of new tabs
  useEffect(() => {
    localStorage.setItem(LOCAL_STORAGE_KEY, JSON.stringify(state));
    if (isLeader && managerRef.current) {
      managerRef.current.broadcastState(state);
    }
  }, [state, isLeader]);

  const dispatch = (action: Action) => {
    if (managerRef.current) {
      managerRef.current.dispatchAction(action);
    } else {
      dispatchLocal(action);
    }
  };

  return (
    <SharedStateContext.Provider value={{ state, isLeader, dispatch }}>
      {children}
    </SharedStateContext.Provider>
  );
}

export function useSharedState() {
  const context = useContext(SharedStateContext);
  if (context === undefined) {
    throw new Error('useSharedState must be used within a SharedStateProvider');
  }
  return context;
}
