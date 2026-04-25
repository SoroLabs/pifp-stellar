import React, { createContext } from 'react';

export const WebSocketContext = createContext({});
export const WebSocketProvider: React.FC<{children: React.ReactNode}> = ({ children }) => {
    return (
        <WebSocketContext.Provider value={{}}>
            {children}
        </WebSocketContext.Provider>
    );
};
