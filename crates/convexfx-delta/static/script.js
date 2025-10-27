// ===== Global State =====
let currentUser = null;
let currentBalances = {};

// ===== Initialization =====
document.addEventListener('DOMContentLoaded', function() {
    // Pre-populate demo users
    ['alice', 'bob', 'charlie'].forEach(user => {
        addUserToSelector(user);
    });
    
    // Load pool liquidity
    fetchPoolData();
    
    // Refresh pool data every 5 seconds
    setInterval(fetchPoolData, 5000);
    
    // Smooth scroll for navigation links
    document.querySelectorAll('a[href^="#"]').forEach(anchor => {
        anchor.addEventListener('click', function (e) {
            e.preventDefault();
            const target = document.querySelector(this.getAttribute('href'));
            if (target) {
                target.scrollIntoView({
                    behavior: 'smooth',
                    block: 'start'
                });
            }
        });
    });
});

// ===== Toast Notifications =====
function showToast(title, message, type = 'info') {
    const container = document.getElementById('toastContainer');
    const toast = document.createElement('div');
    toast.className = `toast ${type}`;
    
    const icons = {
        success: '✅',
        error: '❌',
        info: 'ℹ️',
        warning: '⚠️'
    };
    
    toast.innerHTML = `
        <div class="toast-icon">${icons[type]}</div>
        <div class="toast-content">
            <div class="toast-title">${title}</div>
            <div class="toast-message">${message}</div>
        </div>
    `;
    
    container.appendChild(toast);
    
    setTimeout(() => {
        toast.style.animation = 'slideIn 0.3s ease-out reverse';
        setTimeout(() => toast.remove(), 300);
    }, 5000);
}

// ===== Health Check =====
async function checkHealth() {
    try {
        const response = await fetch('/api/health');
        const data = await response.json();
        
        if (data.success) {
            showToast('System Status', data.data, 'success');
        } else {
            showToast('System Error', data.error || 'Unknown error', 'error');
        }
    } catch (error) {
        showToast('Connection Error', 'Failed to check system health', 'error');
    }
}

// ===== Pool Data =====
async function fetchPoolData() {
    try {
        const response = await fetch('/api/pool');
        const data = await response.json();
        
        if (data.success && data.data) {
            renderPoolData(data.data);
        } else {
            document.getElementById('poolGrid').innerHTML = 
                '<div class="error-message">Failed to load pool data</div>';
        }
    } catch (error) {
        console.error('Failed to fetch pool data:', error);
    }
}

function renderPoolData(poolData) {
    const poolGrid = document.getElementById('poolGrid');
    const assets = poolData.assets || [];
    
    if (assets.length === 0) {
        poolGrid.innerHTML = '<div class="info-message">No liquidity in pool</div>';
        return;
    }
    
    let totalValueUSD = 0;
    assets.forEach(a => totalValueUSD += a.value_usd);
    
    poolGrid.innerHTML = `
        <div class="pool-summary-card">
            <div class="pool-summary-label">Total Pool Value</div>
            <div class="pool-summary-value">$${totalValueUSD.toLocaleString('en-US', {minimumFractionDigits: 2, maximumFractionDigits: 2})}</div>
        </div>
        ${assets.map(asset => `
            <div class="pool-asset-card">
                <div class="pool-asset-symbol">${asset.asset}</div>
                <div class="pool-asset-amount">${asset.amount.toLocaleString('en-US', {minimumFractionDigits: 2, maximumFractionDigits: 2})}</div>
                <div class="pool-asset-value">≈ $${asset.value_usd.toLocaleString('en-US', {minimumFractionDigits: 2, maximumFractionDigits: 2})}</div>
            </div>
        `).join('')}
    `;
}

// ===== User Management =====
function addUserToSelector(userId) {
    const selector = document.getElementById('userSelector');
    const option = document.createElement('option');
    option.value = userId;
    option.textContent = userId;
    selector.appendChild(option);
}

async function registerUser() {
    const userId = document.getElementById('newUserId').value.trim();
    
    if (!userId) {
        showToast('Validation Error', 'Please enter a username', 'warning');
        return;
    }
    
    if (!/^[a-zA-Z0-9_]{3,20}$/.test(userId)) {
        showToast('Validation Error', 'Username must be 3-20 alphanumeric characters', 'warning');
        return;
    }
    
    try {
        const response = await fetch('/api/user/register', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ user_id: userId })
        });
        
        const data = await response.json();
        
        if (data.success) {
            showToast('Registration', `User ${userId} registered successfully`, 'success');
            addUserToSelector(userId);
            document.getElementById('newUserId').value = '';
            
            // Auto-select the new user
            document.getElementById('userSelector').value = userId;
            await onUserSelected();
        } else {
            showToast('Registration Error', data.error || 'Failed to register user', 'error');
        }
    } catch (error) {
        showToast('Registration Error', error.message, 'error');
    }
}

async function onUserSelected() {
    const userId = document.getElementById('userSelector').value;
    
    if (!userId) {
        currentUser = null;
        currentBalances = {};
        document.getElementById('balanceSummary').innerHTML = '<span class="balance-text">No user selected</span>';
        document.getElementById('fromBalance').textContent = '-';
        document.getElementById('toBalance').textContent = '-';
        document.getElementById('balanceSection').style.display = 'none';
        return;
    }
    
    try {
        const response = await fetch(`/api/user/${userId}`);
        const data = await response.json();
        
        if (data.success) {
            currentUser = userId;
            currentBalances = data.data.balances;
            
            // Update summary
            const assetCount = Object.keys(currentBalances).length;
            document.getElementById('balanceSummary').innerHTML = 
                `<span class="balance-text">Logged in as <strong>${userId}</strong> (${assetCount} assets) | 
                <a href="#" onclick="toggleBalanceSection(); return false;">View Details</a></span>`;
            
            // Update balance displays
            updateBalanceDisplays();
            
            // Update detailed balance view
            updateDetailedBalance();
        } else {
            showToast('Error', data.error || 'Failed to load user', 'error');
        }
    } catch (error) {
        showToast('Error', 'Failed to load user data', 'error');
    }
}

function updateBalanceDisplays() {
    const fromAsset = document.getElementById('fromAsset').value;
    const toAsset = document.getElementById('toAsset').value;
    
    document.getElementById('fromBalance').textContent = 
        (currentBalances[fromAsset] || 0).toLocaleString();
    document.getElementById('toBalance').textContent = 
        (currentBalances[toAsset] || 0).toLocaleString();
}

function updateDetailedBalance() {
    const display = document.getElementById('balanceDisplay');
    
    if (!currentBalances || Object.keys(currentBalances).length === 0) {
        display.innerHTML = '<p>No balances found</p>';
        return;
    }
    
    let html = '<div class="balance-grid">';
    for (const [asset, amount] of Object.entries(currentBalances)) {
        html += `
            <div class="balance-item">
                <span class="balance-asset">${asset}</span>
                <span class="balance-amount">${amount.toLocaleString()}</span>
            </div>
        `;
    }
    html += '</div>';
    
    display.innerHTML = html;
}

function toggleBalanceSection() {
    const section = document.getElementById('balanceSection');
    section.style.display = section.style.display === 'none' ? 'block' : 'none';
}

// ===== Trading =====
function swapAssets() {
    const fromAsset = document.getElementById('fromAsset');
    const toAsset = document.getElementById('toAsset');
    
    const temp = fromAsset.value;
    fromAsset.value = toAsset.value;
    toAsset.value = temp;
    
    updateBalanceDisplays();
    updateTradePreview();
}

async function updateTradePreview() {
    const amount = parseFloat(document.getElementById('fromAmount').value);
    const fromAsset = document.getElementById('fromAsset').value;
    const toAsset = document.getElementById('toAsset').value;
    
    // Update balance displays when assets change
    updateBalanceDisplays();
    
    if (!amount || amount <= 0) {
        document.getElementById('toAmount').value = '';
        document.getElementById('priceImpact').textContent = '-';
        document.getElementById('exchangeRate').textContent = '-';
        return;
    }
    
    if (fromAsset === toAsset) {
        showToast('Invalid Trade', 'Cannot trade the same asset', 'warning');
        return;
    }
    
    try {
        const response = await fetch('/api/trade/preview', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                user_id: currentUser || 'demo',
                from_asset: fromAsset,
                to_asset: toAsset,
                amount: amount
            })
        });
        
        const data = await response.json();
        
        if (data.success) {
            document.getElementById('toAmount').value = data.data.to_amount.toFixed(2);
            document.getElementById('priceImpact').textContent = 
                data.data.price_impact.toFixed(3) + '%';
            
            const rate = (data.data.to_amount / amount).toFixed(6);
            document.getElementById('exchangeRate').textContent = 
                `1 ${fromAsset} = ${rate} ${toAsset}`;
        } else {
            showToast('Preview Error', data.error || 'Failed to preview trade', 'error');
        }
    } catch (error) {
        console.error('Error previewing trade:', error);
        showToast('Preview Error', 'Failed to calculate trade preview', 'error');
    }
}

async function executeTrade() {
    const amount = parseFloat(document.getElementById('fromAmount').value);
    const fromAsset = document.getElementById('fromAsset').value;
    const toAsset = document.getElementById('toAsset').value;
    
    if (!currentUser) {
        showToast('User Required', 'Please select or register a user first', 'warning');
        return;
    }
    
    if (!amount || amount <= 0) {
        showToast('Validation Error', 'Please enter a valid amount', 'warning');
        return;
    }
    
    if (fromAsset === toAsset) {
        showToast('Invalid Trade', 'Cannot trade the same asset', 'warning');
        return;
    }
    
    // Check balance
    if (currentBalances[fromAsset] < amount) {
        showToast('Insufficient Balance', `You only have ${currentBalances[fromAsset]} ${fromAsset}`, 'error');
        return;
    }
    
    try {
        showToast('Processing', 'Executing trade...', 'info');
        
        // In a real implementation, this would call a trade execution endpoint
        // For now, we'll simulate success
        setTimeout(async () => {
            showToast('Trade Executed', 
                `Successfully traded ${amount} ${fromAsset} for ${toAsset}`, 
                'success');
            
            // Clear form
            document.getElementById('fromAmount').value = '';
            document.getElementById('toAmount').value = '';
            document.getElementById('priceImpact').textContent = '-';
            document.getElementById('exchangeRate').textContent = '-';
            
            // Reload balance
            if (currentUser) {
                await onUserSelected();
            }
        }, 1000);
    } catch (error) {
        showToast('Trade Error', error.message, 'error');
    }
}

