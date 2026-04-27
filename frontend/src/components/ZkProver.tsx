import React, { useState, useEffect, useRef } from 'react';

export const ZkProver: React.FC = () => {
    const [progress, setProgress] = useState(0);
    const [isGenerating, setIsGenerating] = useState(false);
    const [proof, setProof] = useState<string | null>(null);
    const workerRef = useRef<Worker | null>(null);

    useEffect(() => {
        // Initialize the worker
        workerRef.current = new Worker(
            new URL('../workers/zkProver.worker.ts', import.meta.url),
            { type: 'module' }
        );

        workerRef.current.onmessage = (e) => {
            const { type, progress, proof } = e.data;
            if (type === 'PROGRESS') {
                setProgress(progress);
            } else if (type === 'RESULT') {
                setProof(proof);
                setIsGenerating(false);
            }
        };

        return () => {
            workerRef.current?.terminate();
        };
    }, []);

    const generateProof = () => {
        setProof(null);
        setProgress(0);
        setIsGenerating(true);
        workerRef.current?.postMessage({ type: 'GENERATE_PROOF' });
    };

    return (
        <div className="did-wallet">
            <p className="eyebrow">Privacy Infrastructure</p>
            <h2>WASM ZK Proof Generation</h2>
            <p className="subhead">
                Local proof generation running in a dedicated Web Worker to maintain UI responsiveness.
            </p>
            
            <div className="section" style={{ marginTop: '1.5rem' }}>
                <button 
                    onClick={generateProof} 
                    disabled={isGenerating}
                    className={isGenerating ? 'loading' : ''}
                >
                    {isGenerating ? 'Generating Proof...' : 'Generate Privacy Proof'}
                </button>

                {isGenerating && (
                    <div style={{ marginTop: '1.5rem' }}>
                        <div style={{ 
                            display: 'flex', 
                            justifyContent: 'space-between', 
                            marginBottom: '0.5rem',
                            fontSize: '0.8rem',
                            fontWeight: 'bold',
                            color: '#4f6a76'
                        }}>
                            <span>Computation Progress</span>
                            <span>{progress}%</span>
                        </div>
                        <div style={{ 
                            height: '8px', 
                            background: '#ecf1f4', 
                            borderRadius: '4px', 
                            overflow: 'hidden' 
                        }}>
                            <div style={{ 
                                height: '100%', 
                                width: `${progress}%`, 
                                background: '#1f5868', 
                                transition: 'width 0.3s ease' 
                            }} />
                        </div>
                    </div>
                )}

                {proof && (
                    <div className="result" style={{ marginTop: '1.5rem' }}>
                        <h3>Generated Proof</h3>
                        <pre className="success">{proof}</pre>
                    </div>
                )}
            </div>
        </div>
    );
};
