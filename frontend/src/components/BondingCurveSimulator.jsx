import React, { useEffect, useRef, useState } from 'react';
import { createChart } from 'lightweight-charts';

const BondingCurveSimulator = () => {
  const chartContainerRef = useRef();
  const chartRef = useRef();
  const lineSeriesRef = useRef();
  const pointSeriesRef = useRef();

  const [tradeAmount, setTradeAmount] = useState(10000);
  const [isBuy, setIsBuy] = useState(true);
  const [impact, setImpact] = useState(null);

  // Configuration (Default values)
  const reserve = 1000000;
  const supply = 10000000;
  const ratio = 0.5;

  const calculatePrice = (s) => {
    // P = R / (S * F)
    // For simplicity, we assume R moves with S based on the curve integral
    // P(s) = (P0 / S0^((1-F)/F)) * s^((1-F)/F)
    const p0 = reserve / (supply * ratio);
    return p0 * Math.pow(s / supply, (1 - ratio) / ratio);
  };

  useEffect(() => {
    chartRef.current = createChart(chartContainerRef.current, {
      width: chartContainerRef.current.clientWidth,
      height: 400,
      layout: {
        background: { color: '#0f172a' },
        textColor: '#94a3b8',
      },
      grid: {
        vertLines: { color: '#1e293b' },
        horzLines: { color: '#1e293b' },
      },
      timeScale: {
        visible: false, // We use Supply on X axis instead of Time
      },
    });

    lineSeriesRef.current = chartRef.current.addLineSeries({
      color: '#3b82f6',
      lineWidth: 2,
    });

    pointSeriesRef.current = chartRef.current.addScatterSeries({
      color: '#f59e0b',
      markerSize: 8,
    });

    // Generate curve data
    const data = [];
    for (let s = supply * 0.5; s <= supply * 2; s += supply * 0.05) {
      data.push({ time: s, value: calculatePrice(s) });
    }
    lineSeriesRef.current.setData(data);

    const handleResize = () => {
      chartRef.current.applyOptions({ width: chartContainerRef.current.clientWidth });
    };

    window.addEventListener('resize', handleResize);
    return () => {
      window.removeEventListener('resize', handleResize);
      chartRef.current.remove();
    };
  }, []);

  useEffect(() => {
    const fetchSimulation = async () => {
      try {
        const params = new URLSearchParams({
          amount: tradeAmount,
          is_buy: isBuy,
          reserve,
          supply,
          ratio
        });
        const res = await fetch(`http://localhost:9090/api/bonding/simulate?${params}`);
        const data = await res.json();
        setImpact(data);

        // Update point on chart
        pointSeriesRef.current.setData([
          { time: supply, value: data.current_price },
          { time: isBuy ? supply + data.output_amount : supply - tradeAmount, value: data.new_price }
        ]);
      } catch (err) {
        console.error("Simulation failed:", err);
      }
    };

    fetchSimulation();
  }, [tradeAmount, isBuy]);

  return (
    <div className="bonding-curve-container">
      <div className="simulator-header">
        <h2>Bonding Curve Pricing Engine</h2>
        <p>Simulate trade impact and slippage on the continuous token supply.</p>
      </div>

      <div className="main-layout">
        <div className="chart-wrapper" ref={chartContainerRef} />
        
        <div className="controls-panel">
          <div className="control-group">
            <label>Trade Direction</label>
            <div className="toggle-buttons">
              <button 
                className={isBuy ? 'active' : ''} 
                onClick={() => setIsBuy(true)}
              >
                Buy (Deposit Reserve)
              </button>
              <button 
                className={!isBuy ? 'active sell' : ''} 
                onClick={() => setIsBuy(false)}
              >
                Sell (Burn Supply)
              </button>
            </div>
          </div>

          <div className="control-group">
            <label>Amount: {tradeAmount.toLocaleString()} {isBuy ? 'USDC' : 'TOKENS'}</label>
            <input 
              type="range" 
              min="100" 
              max="100000" 
              step="100" 
              value={tradeAmount} 
              onChange={(e) => setTradeAmount(Number(e.target.value))}
            />
          </div>

          {impact && (
            <div className="stats-grid">
              <div className="stat-card">
                <span className="label">Expected Output</span>
                <span className="value">{impact.output_amount.toFixed(2)}</span>
              </div>
              <div className="stat-card">
                <span className="label">Price Impact</span>
                <span className="value highlight">{(impact.price_impact * 100).toFixed(2)}%</span>
              </div>
              <div className="stat-card">
                <span className="label">Slippage</span>
                <span className="value">{(impact.slippage * 100).toFixed(2)}%</span>
              </div>
              <div className="stat-card">
                <span className="label">Next Price</span>
                <span className="value">${impact.new_price.toFixed(4)}</span>
              </div>
            </div>
          )}
        </div>
      </div>

      <style jsx>{`
        .bonding-curve-container {
          padding: 24px;
          background: #0f172a;
          border-radius: 16px;
          border: 1px solid #1e293b;
          color: #f8fafc;
        }
        .simulator-header h2 { margin: 0; color: #3b82f6; }
        .simulator-header p { color: #94a3b8; font-size: 0.9rem; margin: 8px 0 24px 0; }
        .main-layout { display: grid; grid-template-columns: 1fr 350px; gap: 24px; }
        .chart-wrapper { height: 400px; border: 1px solid #1e293b; border-radius: 12px; overflow: hidden; }
        .controls-panel { display: flex; flex-direction: column; gap: 20px; }
        .control-group label { display: block; margin-bottom: 8px; font-size: 0.85rem; color: #94a3b8; font-weight: 600; }
        .toggle-buttons { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
        .toggle-buttons button { 
          padding: 10px; border-radius: 8px; border: 1px solid #1e293b; background: #1e293b; color: #94a3b8; cursor: pointer; font-weight: 600; transition: all 0.2s;
        }
        .toggle-buttons button.active { background: #3b82f6; color: white; border-color: #3b82f6; }
        .toggle-buttons button.active.sell { background: #ef4444; border-color: #ef4444; }
        input[type="range"] { width: 100%; accent-color: #3b82f6; }
        .stats-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
        .stat-card { background: #1e293b; padding: 12px; border-radius: 10px; border: 1px solid #334155; }
        .stat-card .label { display: block; font-size: 0.75rem; color: #64748b; margin-bottom: 4px; }
        .stat-card .value { font-size: 1.1rem; font-weight: 700; color: #f8fafc; }
        .stat-card .value.highlight { color: #f59e0b; }
      `}</style>
    </div>
  );
};

export default BondingCurveSimulator;
