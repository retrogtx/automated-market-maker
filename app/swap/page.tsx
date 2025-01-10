'use client';

import { useState } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';

export default function SwapPage() {
  const { connected } = useWallet();
  const [amount, setAmount] = useState('');
  const [expectedOutput, setExpectedOutput] = useState('0');

  const handleSwap = async () => {
    // Implement swap logic here using your AMM program
    console.log('Swapping tokens...');
  };

  return (
    <main className="min-h-screen bg-gray-900 text-white p-8">
      <div className="max-w-lg mx-auto">
        <div className="flex justify-between items-center mb-8">
          <h1 className="text-3xl font-bold">Swap Tokens</h1>
          <WalletMultiButton />
        </div>

        {connected ? (
          <div className="bg-gray-800 p-6 rounded-lg">
            <div className="mb-4">
              <label className="block text-sm font-medium mb-2">
                Amount to Swap
              </label>
              <input
                type="number"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                className="w-full bg-gray-700 p-3 rounded-lg text-white"
                placeholder="Enter amount"
              />
            </div>

            <div className="mb-6">
              <label className="block text-sm font-medium mb-2">
                Expected Output
              </label>
              <div className="bg-gray-700 p-3 rounded-lg">
                {expectedOutput} tokens
              </div>
            </div>

            <button
              onClick={handleSwap}
              className="w-full bg-purple-600 hover:bg-purple-700 text-white font-bold py-3 px-4 rounded-lg transition"
            >
              Swap
            </button>
          </div>
        ) : (
          <div className="text-center py-12">
            <h2 className="text-2xl font-semibold mb-4">
              Connect your wallet to swap tokens
            </h2>
          </div>
        )}
      </div>
    </main>
  );
} 