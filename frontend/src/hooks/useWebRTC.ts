import { useEffect, useRef, useState, useCallback } from 'react';
import { P2PMessage, SignalingMessage } from './types';

const SIGNALING_URL = 'ws://localhost:3001/ws';

export function useWebRTC(roomId: string, isInitiator: boolean) {
    const [messages, setMessages] = useState<P2PMessage[]>([]);
    const [connectionStatus, setConnectionStatus] = useState<'disconnected' | 'connecting' | 'connected'>('disconnected');
    
    const pcRef = useRef<RTCPeerConnection | null>(null);
    const dcRef = useRef<RTCDataChannel | null>(null);
    const wsRef = useRef<WebSocket | null>(null);

    const sendMessage = useCallback((msg: P2PMessage) => {
        if (dcRef.current && dcRef.current.readyState === 'open') {
            dcRef.current.send(JSON.stringify(msg));
            setMessages(prev => [...prev, msg]);
        }
    }, []);

    useEffect(() => {
        const ws = new WebSocket(SIGNALING_URL);
        wsRef.current = ws;

        const pc = new RTCPeerConnection({
            iceServers: [{ urls: 'stun:stun.l.google.com:19302' }]
        });
        pcRef.current = pc;

        ws.onopen = () => {
            console.log('Connected to signaling server');
            ws.send(JSON.stringify({ type: 'join', roomId }));
            setConnectionStatus('connecting');
        };

        ws.onmessage = async (event) => {
            const msg: SignalingMessage = JSON.parse(event.data);
            if (msg.type === 'signal') {
                const { data } = msg;
                if (data.sdp) {
                    await pc.setRemoteDescription(new RTCSessionDescription(data.sdp));
                    if (data.sdp.type === 'offer') {
                        const answer = await pc.createAnswer();
                        await pc.setLocalDescription(answer);
                        ws.send(JSON.stringify({
                            type: 'signal',
                            roomId,
                            data: { sdp: pc.localDescription }
                        }));
                    }
                } else if (data.candidate) {
                    await pc.addIceCandidate(new RTCIceCandidate(data.candidate));
                }
            }
        };

        pc.onicecandidate = (event) => {
            if (event.candidate) {
                ws.send(JSON.stringify({
                    type: 'signal',
                    roomId,
                    data: { candidate: event.candidate }
                }));
            }
        };

        const setupDataChannel = (channel: RTCDataChannel) => {
            dcRef.current = channel;
            channel.onopen = () => {
                console.log('Data channel opened');
                setConnectionStatus('connected');
            };
            channel.onmessage = (event) => {
                const msg: P2PMessage = JSON.parse(event.data);
                setMessages(prev => [...prev, msg]);
            };
            channel.onclose = () => setConnectionStatus('disconnected');
        };

        if (isInitiator) {
            const dc = pc.createDataChannel('chat');
            setupDataChannel(dc);
            
            pc.createOffer().then(async (offer) => {
                await pc.setLocalDescription(offer);
                ws.send(JSON.stringify({
                    type: 'signal',
                    roomId,
                    data: { sdp: offer }
                }));
            });
        } else {
            pc.ondatachannel = (event) => {
                setupDataChannel(event.channel);
            };
        }

        return () => {
            pc.close();
            ws.close();
        };
    }, [roomId, isInitiator]);

    return { messages, sendMessage, connectionStatus };
}
