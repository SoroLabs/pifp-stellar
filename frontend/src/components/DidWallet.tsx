import React, { useState } from 'react';
import './DidWallet.css';

export const DidWallet: React.FC = () => {
    const [credential, setCredential] = useState<string | null>(null);
    const [userId] = useState('user-123');
    const [requestedClaims, setRequestedClaims] = useState<string[]>(['isHuman']);
    const [result, setResult] = useState<any>(null);
    const [loading, setLoading] = useState(false);

    const getCredential = async () => {
        setLoading(true);
        try {
            // 1. Request Challenge
            const challengeRes = await fetch('http://localhost:9090/did/challenge', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ user_id: userId })
            });
            const { challenge } = await challengeRes.json();

            // 2. Issue with Challenge
            const res = await fetch('http://localhost:9090/did/issue', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    user_id: userId,
                    challenge: challenge,
                    claims: { isHuman: true, age: 22, residency: 'US' }
                })
            });
            const data = await res.json();
            setCredential(data.credential);
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    };

    const verify = async () => {
        if (!credential) return;
        setLoading(true);
        try {
            const res = await fetch('http://localhost:9090/did/verify', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    credential,
                    requested_claims: requestedClaims
                })
            });
            const data = await res.json();
            setResult(data);
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    };

    const toggleClaim = (claim: string) => {
        setRequestedClaims(prev => 
            prev.includes(claim) ? prev.filter(c => c !== claim) : [...prev, claim]
        );
    };

    return (
        <div className="did-wallet">
            <h2>PIFP Identity Verification</h2>
            
            <div className="section">
                {!credential ? (
                    <button onClick={getCredential} disabled={loading}>
                        {loading ? 'Authenticating...' : 'Get Credential'}
                    </button>
                ) : (
                    <p className="state success">Identity Verified via did:pifp:oracle</p>
                )}
            </div>

            {credential && (
                <div className="section">
                    <h3>Selective Disclosure & Derived Proofs</h3>
                    <div className="field-selector">
                        {['isHuman', 'age', 'isAgeOver18'].map(claim => (
                            <label key={claim}>
                                <input 
                                    type="checkbox" 
                                    checked={requestedClaims.includes(claim)}
                                    onChange={() => toggleClaim(claim)}
                                />
                                {claim}
                            </label>
                        ))}
                    </div>
                    <button onClick={verify} disabled={loading}>
                        {loading ? 'Verifying...' : 'Verify Selected Claims'}
                    </button>
                </div>
            )}

            {result && (
                <div className="section result">
                    <h3>Result</h3>
                    <pre className={result.valid ? 'success' : 'error'}>
                        {JSON.stringify(result.disclosed, null, 2)}
                    </pre>
                </div>
            )}
        </div>
    );
};
