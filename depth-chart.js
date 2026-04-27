class DepthChart {
    constructor(canvasId) {
        this.canvas = document.getElementById(canvasId);
        this.ctx = this.canvas.getContext('2d', { 
            alpha: false,
            desynchronized: true 
        });
        
        // Performance tracking
        this.frameCount = 0;
        this.lastFrameTime = performance.now();
        this.renderTime = 0;
        this.mouseUpdateTime = 0;
        
        // Dirty rectangle optimization
        this.dirtyRegions = [];
        this.fullRedraw = true;
        
        // Chart data
        this.bids = [];
        this.asks = [];
        this.scaleType = 'linear';
        
        // Mouse tracking
        this.mouseX = 0;
        this.mouseY = 0;
        this.mousePrice = 0;
        this.mouseVolume = 0;
        this.showCrosshair = false;
        
        // Animation
        this.animationEnabled = true;
        this.animationFrame = null;
        
        // Chart dimensions
        this.padding = { top: 40, right: 80, bottom: 60, left: 80 };
        this.gridColor = '#2a2a2a';
        this.textColor = '#999';
        this.bidColor = '#00ff88';
        this.askColor = '#ff4444';
        
        this.setupCanvas();
        this.generateSampleData();
        this.setupEventListeners();
        this.startRenderLoop();
    }
    
    setupCanvas() {
        const container = this.canvas.parentElement;
        const rect = container.getBoundingClientRect();
        
        this.canvas.width = rect.width;
        this.canvas.height = rect.height;
        
        // Enable crisp rendering
        this.ctx.imageSmoothingEnabled = false;
        this.ctx.textBaseline = 'middle';
        this.ctx.textAlign = 'center';
        
        // Calculate chart area
        this.chartWidth = this.canvas.width - this.padding.left - this.padding.right;
        this.chartHeight = this.canvas.height - this.padding.top - this.padding.bottom;
    }
    
    generateSampleData(numPoints = 1000) {
        const basePrice = 50000;
        const spread = 100;
        
        // Generate bids (buy orders)
        this.bids = [];
        let cumulativeBidVolume = 0;
        for (let i = 0; i < numPoints; i++) {
            const price = basePrice - (i * spread / numPoints);
            const volume = Math.random() * 10 + 0.1;
            cumulativeBidVolume += volume;
            this.bids.push({
                price: price,
                volume: volume,
                cumulativeVolume: cumulativeBidVolume
            });
        }
        
        // Generate asks (sell orders)
        this.asks = [];
        let cumulativeAskVolume = 0;
        for (let i = 0; i < numPoints; i++) {
            const price = basePrice + (i * spread / numPoints);
            const volume = Math.random() * 10 + 0.1;
            cumulativeAskVolume += volume;
            this.asks.push({
                price: price,
                volume: volume,
                cumulativeVolume: cumulativeAskVolume
            });
        }
        
        this.invalidateChart();
    }
    
    // Mathematical mapping functions
    mapPriceToX(price) {
        const minPrice = Math.min(this.bids[0]?.price || 0, this.asks[0]?.price || 0);
        const maxPrice = Math.max(this.bids[this.bids.length - 1]?.price || 0, 
                                 this.asks[this.asks.length - 1]?.price || 0);
        
        if (this.scaleType === 'logarithmic') {
            const logMin = Math.log10(Math.max(minPrice, 1));
            const logMax = Math.log10(Math.max(maxPrice, 1));
            const logPrice = Math.log10(Math.max(price, 1));
            return this.padding.left + ((logPrice - logMin) / (logMax - logMin)) * this.chartWidth;
        } else {
            return this.padding.left + ((price - minPrice) / (maxPrice - minPrice)) * this.chartWidth;
        }
    }
    
    mapVolumeToY(volume) {
        const maxVolume = Math.max(
            this.bids[this.bids.length - 1]?.cumulativeVolume || 0,
            this.asks[this.asks.length - 1]?.cumulativeVolume || 0
        );
        
        if (this.scaleType === 'logarithmic') {
            const logMax = Math.log10(Math.max(maxVolume, 1));
            const logVolume = Math.log10(Math.max(volume, 1));
            return this.canvas.height - this.padding.bottom - (logVolume / logMax) * this.chartHeight;
        } else {
            return this.canvas.height - this.padding.bottom - (volume / maxVolume) * this.chartHeight;
        }
    }
    
    mapXToPrice(x) {
        const minPrice = Math.min(this.bids[0]?.price || 0, this.asks[0]?.price || 0);
        const maxPrice = Math.max(this.bids[this.bids.length - 1]?.price || 0, 
                                 this.asks[this.asks.length - 1]?.price || 0);
        
        if (this.scaleType === 'logarithmic') {
            const logMin = Math.log10(Math.max(minPrice, 1));
            const logMax = Math.log10(Math.max(maxPrice, 1));
            const logPrice = logMin + ((x - this.padding.left) / this.chartWidth) * (logMax - logMin);
            return Math.pow(10, logPrice);
        } else {
            return minPrice + ((x - this.padding.left) / this.chartWidth) * (maxPrice - minPrice);
        }
    }
    
    mapYToVolume(y) {
        const maxVolume = Math.max(
            this.bids[this.bids.length - 1]?.cumulativeVolume || 0,
            this.asks[this.asks.length - 1]?.cumulativeVolume || 0
        );
        
        if (this.scaleType === 'logarithmic') {
            const logMax = Math.log10(Math.max(maxVolume, 1));
            const logVolume = ((this.canvas.height - this.padding.bottom - y) / this.chartHeight) * logMax;
            return Math.pow(10, logVolume);
        } else {
            return ((this.canvas.height - this.padding.bottom - y) / this.chartHeight) * maxVolume;
        }
    }
    
    // Dirty rectangle optimization
    invalidateRegion(x, y, width, height) {
        this.dirtyRegions.push({ x, y, width, height });
    }
    
    invalidateChart() {
        this.fullRedraw = true;
        this.dirtyRegions = [];
    }
    
    // Optimized rendering loop
    startRenderLoop() {
        const render = (timestamp) => {
            const startTime = performance.now();
            
            // Clear only dirty regions or full canvas
            if (this.fullRedraw) {
                this.ctx.fillStyle = '#111111';
                this.ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
                this.renderChart();
                this.fullRedraw = false;
                this.dirtyRegions = [];
            } else {
                // Clear dirty regions
                this.dirtyRegions.forEach(region => {
                    this.ctx.fillStyle = '#111111';
                    this.ctx.fillRect(region.x, region.y, region.width, region.height);
                });
                
                // Re-render affected areas
                this.renderChart();
                this.dirtyRegions = [];
            }
            
            // Render crosshair if mouse is over chart
            if (this.showCrosshair) {
                this.renderCrosshair();
                this.renderTooltip();
            }
            
            // Update performance stats
            this.renderTime = performance.now() - startTime;
            this.updatePerformanceStats();
            
            if (this.animationEnabled) {
                this.animationFrame = requestAnimationFrame(render);
            }
        };
        
        render(performance.now());
    }
    
    renderChart() {
        this.renderGrid();
        this.renderAxes();
        this.renderDepthData();
    }
    
    renderGrid() {
        this.ctx.strokeStyle = this.gridColor;
        this.ctx.lineWidth = 1;
        
        // Vertical grid lines
        for (let i = 0; i <= 10; i++) {
            const x = this.padding.left + (i / 10) * this.chartWidth;
            this.ctx.beginPath();
            this.ctx.moveTo(x, this.padding.top);
            this.ctx.lineTo(x, this.canvas.height - this.padding.bottom);
            this.ctx.stroke();
        }
        
        // Horizontal grid lines
        for (let i = 0; i <= 10; i++) {
            const y = this.padding.top + (i / 10) * this.chartHeight;
            this.ctx.beginPath();
            this.ctx.moveTo(this.padding.left, y);
            this.ctx.lineTo(this.canvas.width - this.padding.right, y);
            this.ctx.stroke();
        }
    }
    
    renderAxes() {
        this.ctx.strokeStyle = this.textColor;
        this.ctx.fillStyle = this.textColor;
        this.ctx.lineWidth = 2;
        this.ctx.font = '11px monospace';
        
        // X-axis
        this.ctx.beginPath();
        this.ctx.moveTo(this.padding.left, this.canvas.height - this.padding.bottom);
        this.ctx.lineTo(this.canvas.width - this.padding.right, this.canvas.height - this.padding.bottom);
        this.ctx.stroke();
        
        // Y-axis
        this.ctx.beginPath();
        this.ctx.moveTo(this.padding.left, this.padding.top);
        this.ctx.lineTo(this.padding.left, this.canvas.height - this.padding.bottom);
        this.ctx.stroke();
        
        // Price labels
        const minPrice = Math.min(this.bids[0]?.price || 0, this.asks[0]?.price || 0);
        const maxPrice = Math.max(this.bids[this.bids.length - 1]?.price || 0, 
                                 this.asks[this.asks.length - 1]?.price || 0);
        
        for (let i = 0; i <= 5; i++) {
            const price = minPrice + (i / 5) * (maxPrice - minPrice);
            const x = this.mapPriceToX(price);
            
            this.ctx.save();
            this.ctx.textAlign = 'center';
            this.ctx.fillText(price.toFixed(2), x, this.canvas.height - this.padding.bottom + 20);
            this.ctx.restore();
        }
        
        // Volume labels
        const maxVolume = Math.max(
            this.bids[this.bids.length - 1]?.cumulativeVolume || 0,
            this.asks[this.asks.length - 1]?.cumulativeVolume || 0
        );
        
        for (let i = 0; i <= 5; i++) {
            const volume = (i / 5) * maxVolume;
            const y = this.mapVolumeToY(volume);
            
            this.ctx.save();
            this.ctx.textAlign = 'right';
            this.ctx.fillText(volume.toFixed(1), this.padding.left - 10, y);
            this.ctx.restore();
        }
    }
    
    renderDepthData() {
        // Render bids (buy orders) - green
        if (this.bids.length > 0) {
            this.ctx.fillStyle = this.bidColor + '40';
            this.ctx.strokeStyle = this.bidColor;
            this.ctx.lineWidth = 2;
            
            this.ctx.beginPath();
            this.ctx.moveTo(this.mapPriceToX(this.bids[0].price), this.canvas.height - this.padding.bottom);
            
            for (let i = 0; i < this.bids.length; i++) {
                const x = this.mapPriceToX(this.bids[i].price);
                const y = this.mapVolumeToY(this.bids[i].cumulativeVolume);
                this.ctx.lineTo(x, y);
            }
            
            this.ctx.lineTo(this.mapPriceToX(this.bids[this.bids.length - 1].price), 
                          this.canvas.height - this.padding.bottom);
            this.ctx.closePath();
            this.ctx.fill();
            this.ctx.stroke();
        }
        
        // Render asks (sell orders) - red
        if (this.asks.length > 0) {
            this.ctx.fillStyle = this.askColor + '40';
            this.ctx.strokeStyle = this.askColor;
            this.ctx.lineWidth = 2;
            
            this.ctx.beginPath();
            this.ctx.moveTo(this.mapPriceToX(this.asks[0].price), this.canvas.height - this.padding.bottom);
            
            for (let i = 0; i < this.asks.length; i++) {
                const x = this.mapPriceToX(this.asks[i].price);
                const y = this.mapVolumeToY(this.asks[i].cumulativeVolume);
                this.ctx.lineTo(x, y);
            }
            
            this.ctx.lineTo(this.mapPriceToX(this.asks[this.asks.length - 1].price), 
                          this.canvas.height - this.padding.bottom);
            this.ctx.closePath();
            this.ctx.fill();
            this.ctx.stroke();
        }
    }
    
    renderCrosshair() {
        this.ctx.strokeStyle = '#ffffff60';
        this.ctx.lineWidth = 1;
        this.ctx.setLineDash([5, 5]);
        
        // Vertical line
        this.ctx.beginPath();
        this.ctx.moveTo(this.mouseX, this.padding.top);
        this.ctx.lineTo(this.mouseX, this.canvas.height - this.padding.bottom);
        this.ctx.stroke();
        
        // Horizontal line
        this.ctx.beginPath();
        this.ctx.moveTo(this.padding.left, this.mouseY);
        this.ctx.lineTo(this.canvas.width - this.padding.right, this.mouseY);
        this.ctx.stroke();
        
        this.ctx.setLineDash([]);
    }
    
    renderTooltip() {
        const price = this.mapXToPrice(this.mouseX);
        const volume = this.mapYToVolume(this.mouseY);
        
        const tooltipText = `Price: $${price.toFixed(2)}\nVolume: ${volume.toFixed(2)}`;
        const lines = tooltipText.split('\n');
        
        this.ctx.fillStyle = 'rgba(0, 0, 0, 0.8)';
        this.ctx.strokeStyle = '#333';
        this.ctx.lineWidth = 1;
        
        const padding = 8;
        const lineHeight = 14;
        const maxWidth = Math.max(...lines.map(line => this.ctx.measureText(line).width));
        const tooltipWidth = maxWidth + padding * 2;
        const tooltipHeight = lines.length * lineHeight + padding * 2;
        
        let tooltipX = this.mouseX + 15;
        let tooltipY = this.mouseY - tooltipHeight - 15;
        
        // Keep tooltip within canvas bounds
        if (tooltipX + tooltipWidth > this.canvas.width - this.padding.right) {
            tooltipX = this.mouseX - tooltipWidth - 15;
        }
        
        if (tooltipY < this.padding.top) {
            tooltipY = this.mouseY + 15;
        }
        
        // Draw tooltip background
        this.ctx.fillRect(tooltipX, tooltipY, tooltipWidth, tooltipHeight);
        this.ctx.strokeRect(tooltipX, tooltipY, tooltipWidth, tooltipHeight);
        
        // Draw tooltip text
        this.ctx.fillStyle = '#ffffff';
        this.ctx.font = '12px monospace';
        this.ctx.textAlign = 'left';
        this.ctx.textBaseline = 'middle';
        
        lines.forEach((line, index) => {
            this.ctx.fillText(line, tooltipX + padding, tooltipY + padding + (index + 0.5) * lineHeight);
        });
    }
    
    setupEventListeners() {
        // Mouse tracking with sub-5ms latency
        this.canvas.addEventListener('mousemove', (e) => {
            const mouseStartTime = performance.now();
            
            const rect = this.canvas.getBoundingClientRect();
            this.mouseX = e.clientX - rect.left;
            this.mouseY = e.clientY - rect.top;
            
            // Check if mouse is in chart area
            if (this.mouseX >= this.padding.left && 
                this.mouseX <= this.canvas.width - this.padding.right &&
                this.mouseY >= this.padding.top && 
                this.mouseY <= this.canvas.height - this.padding.bottom) {
                this.showCrosshair = true;
                this.mouseUpdateTime = performance.now() - mouseStartTime;
            } else {
                this.showCrosshair = false;
            }
            
            // Invalidate crosshair area only
            this.invalidateRegion(
                Math.max(0, this.mouseX - 100),
                Math.max(0, this.mouseY - 50),
                200,
                100
            );
        });
        
        this.canvas.addEventListener('mouseleave', () => {
            this.showCrosshair = false;
            this.invalidateChart();
        });
        
        // Control listeners
        document.getElementById('scaleType').addEventListener('change', (e) => {
            this.scaleType = e.target.value;
            this.invalidateChart();
        });
        
        document.getElementById('dataPoints').addEventListener('change', (e) => {
            this.generateSampleData(parseInt(e.target.value));
        });
        
        document.getElementById('toggleAnimation').addEventListener('click', () => {
            this.animationEnabled = !this.animationEnabled;
            if (this.animationEnabled) {
                this.startRenderLoop();
            } else {
                cancelAnimationFrame(this.animationFrame);
            }
        });
        
        document.getElementById('regenerateData').addEventListener('click', () => {
            const numPoints = parseInt(document.getElementById('dataPoints').value);
            this.generateSampleData(numPoints);
        });
        
        // Handle window resize
        window.addEventListener('resize', () => {
            this.setupCanvas();
            this.invalidateChart();
        });
    }
    
    updatePerformanceStats() {
        this.frameCount++;
        const currentTime = performance.now();
        const deltaTime = currentTime - this.lastFrameTime;
        
        if (deltaTime >= 1000) {
            const fps = Math.round((this.frameCount * 1000) / deltaTime);
            document.getElementById('fps').textContent = fps;
            this.frameCount = 0;
            this.lastFrameTime = currentTime;
        }
        
        document.getElementById('renderTime').textContent = this.renderTime.toFixed(1);
        document.getElementById('mouseLatency').textContent = this.mouseUpdateTime.toFixed(1);
    }
    
    // Public API for external data updates
    updateData(bids, asks) {
        this.bids = bids;
        this.asks = asks;
        this.invalidateChart();
    }
    
    setScaleType(type) {
        this.scaleType = type;
        this.invalidateChart();
    }
    
    destroy() {
        if (this.animationFrame) {
            cancelAnimationFrame(this.animationFrame);
        }
    }
}

// Initialize the depth chart when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    const chart = new DepthChart('depthChart');
    
    // Make chart globally accessible for debugging
    window.depthChart = chart;
});
