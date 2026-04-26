import React, { useState, useEffect } from 'react';
import { useWebRTC } from '../hooks/useWebRTC';
import { motion, AnimatePresence } from 'framer-motion';
import { MessageSquare, BookOpen, Send, ShieldCheck, Zap } from 'lucide-react';
import { StellarService } from '../services/StellarService';
import { P2PMessage } from '../types';

export const OTCInterface: React.FC<{ roomId: string; isInitiator: boolean }> = ({ roomId, isInitiator }) => {
    const { messages, sendMessage, connectionStatus } = useWebRTC(roomId, isInitiator);
    const [inputText, setInputText] = useState('');
    const [price, setPrice] = useState(0);
    const [amount, setAmount] = useState(0);
    const [walletAddress, setWalletAddress] = useState('');
    const [peerAddress, setPeerAddress] = useState('');

    const handleSendChat = () => {
        if (!inputText.trim()) return;
        sendMessage({ type: 'chat', text: inputText, sender: isInitiator ? 'A' : 'B' });
        setInputText('');
    };

    const handleUpdateOrder = (p: number, a: number) => {
        setPrice(p);
        setAmount(a);
        sendMessage({ type: 'order_update', price: p, amount: a, side: isInitiator ? 'sell' : 'buy' });
    };

    const proposeTransaction = async () => {
        // In a real app, we'd fetch sequence number from Horizon
        const mockSeq = "123456789"; 
        const xdr = await StellarService.constructAtomicSwap(
            walletAddress,
            'XLM',
            amount.toString(),
            peerAddress,
            'native', // Asset B
            (amount * price).toString(),
            mockSeq
        );
        sendMessage({ type: 'transaction_proposal', xdr });
    };

    return (
        <div className="otc-container">
            <div className="otc-header">
                <div className="status-indicator">
                    <div className={`status-dot ${connectionStatus}`} />
                    <span>{connectionStatus.toUpperCase()}</span>
                </div>
                <h2>OTC Negotiation Room: {roomId}</h2>
            </div>

            <div className="otc-grid">
                <section className="chat-section">
                    <div className="chat-header">
                        <MessageSquare size={18} />
                        <h3>Secure Negotiation</h3>
                    </div>
                    <div className="chat-messages">
                        <AnimatePresence>
                            {messages.filter(m => m.type === 'chat').map((m, i) => (
                                <motion.div 
                                    key={i}
                                    initial={{ opacity: 0, y: 10 }}
                                    animate={{ opacity: 1, y: 0 }}
                                    className={`message-bubble ${m.sender === (isInitiator ? 'A' : 'B') ? 'own' : 'peer'}`}
                                >
                                    {m.text}
                                </motion.div>
                            ))}
                        </AnimatePresence>
                    </div>
                    <div className="chat-input">
                        <input 
                            value={inputText} 
                            onChange={e => setInputText(e.target.value)}
                            onKeyPress={e => e.key === 'Enter' && handleSendChat()}
                            placeholder="Type a message..."
                        />
                        <button onClick={handleSendChat}><Send size={18} /></button>
                    </div>
                </section>

                <section className="order-section">
                    <div className="order-header">
                        <BookOpen size={18} />
                        <h3>Order Parameters</h3>
                    </div>
                    <div className="order-form">
                        <div className="input-group">
                            <label>Price (XLM)</label>
                            <input 
                                type="number" 
                                value={price} 
                                onChange={e => handleUpdateOrder(Number(e.target.value), amount)} 
                            />
                        </div>
                        <div className="input-group">
                            <label>Amount (Asset)</label>
                            <input 
                                type="number" 
                                value={amount} 
                                onChange={e => handleUpdateOrder(price, Number(e.target.value))} 
                            />
                        </div>
                        <div className="order-summary">
                            <div className="summary-row">
                                <span>Total Value</span>
                                <span>{(price * amount).toFixed(4)} XLM</span>
                            </div>
                        </div>
                        
                        <div className="wallet-config">
                            <input 
                                placeholder="Your Wallet Address" 
                                value={walletAddress} 
                                onChange={e => setWalletAddress(e.target.value)} 
                            />
                            <input 
                                placeholder="Peer Wallet Address" 
                                value={peerAddress} 
                                onChange={e => setPeerAddress(e.target.value)} 
                            />
                        </div>

                        <button 
                            className="propose-btn"
                            onClick={proposeTransaction}
                            disabled={connectionStatus !== 'connected'}
                        >
                            <ShieldCheck size={18} />
                            Propose Atomic Swap
                        </button>
                    </div>
                </section>
            </div>

            <style>{`
                .otc-container {
                    display: flex;
                    flex-direction: column;
                    height: calc(100vh - 100px);
                    background: #0f172a;
                    color: white;
                    padding: 20px;
                    border-radius: 12px;
                    gap: 20px;
                }
                .otc-header {
                    display: flex;
                    align-items: center;
                    gap: 15px;
                }
                .status-indicator {
                    display: flex;
                    align-items: center;
                    gap: 8px;
                    background: #1e293b;
                    padding: 4px 12px;
                    border-radius: 20px;
                    font-size: 12px;
                    font-weight: 600;
                }
                .status-dot {
                    width: 8px;
                    height: 8px;
                    border-radius: 50%;
                }
                .status-dot.connected { background: #10b981; box-shadow: 0 0 8px #10b981; }
                .status-dot.connecting { background: #f59e0b; }
                .status-dot.disconnected { background: #ef4444; }

                .otc-grid {
                    display: grid;
                    grid-template-columns: 1fr 350px;
                    gap: 20px;
                    flex: 1;
                    min-height: 0;
                }
                .chat-section, .order-section {
                    background: #1e293b;
                    border-radius: 12px;
                    display: flex;
                    flex-direction: column;
                    border: 1px solid #334155;
                }
                .chat-header, .order-header {
                    padding: 15px;
                    border-bottom: 1px solid #334155;
                    display: flex;
                    align-items: center;
                    gap: 10px;
                }
                .chat-messages {
                    flex: 1;
                    overflow-y: auto;
                    padding: 15px;
                    display: flex;
                    flex-direction: column;
                    gap: 10px;
                }
                .message-bubble {
                    padding: 8px 12px;
                    border-radius: 12px;
                    max-width: 80%;
                    font-size: 14px;
                }
                .message-bubble.own {
                    align-self: flex-end;
                    background: #3b82f6;
                }
                .message-bubble.peer {
                    align-self: flex-start;
                    background: #334155;
                }
                .chat-input {
                    padding: 15px;
                    display: flex;
                    gap: 10px;
                    border-top: 1px solid #334155;
                }
                .chat-input input {
                    flex: 1;
                    background: #0f172a;
                    border: 1px solid #334155;
                    color: white;
                    padding: 8px 12px;
                    border-radius: 8px;
                }
                .chat-input button {
                    background: #3b82f6;
                    border: none;
                    color: white;
                    padding: 8px;
                    border-radius: 8px;
                    cursor: pointer;
                }
                .order-form {
                    padding: 20px;
                    display: flex;
                    flex-direction: column;
                    gap: 15px;
                }
                .input-group {
                    display: flex;
                    flex-direction: column;
                    gap: 5px;
                }
                .input-group label {
                    font-size: 12px;
                    color: #94a3b8;
                }
                .input-group input {
                    background: #0f172a;
                    border: 1px solid #334155;
                    color: white;
                    padding: 8px;
                    border-radius: 6px;
                }
                .order-summary {
                    background: #0f172a;
                    padding: 15px;
                    border-radius: 8px;
                    border: 1px dashed #334155;
                }
                .summary-row {
                    display: flex;
                    justify-content: space-between;
                    font-size: 14px;
                }
                .wallet-config {
                    display: flex;
                    flex-direction: column;
                    gap: 10px;
                    margin-top: 10px;
                }
                .wallet-config input {
                    background: #0f172a;
                    border: 1px solid #334155;
                    color: white;
                    padding: 6px;
                    border-radius: 4px;
                    font-size: 12px;
                }
                .propose-btn {
                    margin-top: 10px;
                    background: #10b981;
                    color: white;
                    border: none;
                    padding: 12px;
                    border-radius: 8px;
                    font-weight: 600;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    gap: 8px;
                    cursor: pointer;
                    transition: all 0.2s;
                }
                .propose-btn:disabled { opacity: 0.5; cursor: not-allowed; }
                .propose-btn:hover:not(:disabled) { transform: translateY(-2px); box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3); }
            `}</style>
        </div>
    );
};
