import React from 'react';

export const WasmEditor: React.FC = () => {
    return (
        <div>
            <h2>Wasm Script Editor</h2>
            <textarea placeholder="Write Wasm/Rust code here"></textarea>
            <button>Test Script</button>
        </div>
    );
};
