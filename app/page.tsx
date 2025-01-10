'use client';

import { useWallet } from '@solana/wallet-adapter-react';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import Link from 'next/link';

export default function Home() {
  const { connected } = useWallet();

  return (
    <main className="min-h-screen bg-gray-900 text-white p-8">
      <div className="max-w-4xl mx-auto">
        <div className="flex justify-between items-center mb-8">
          <h1 className="text-3xl font-bold">Solana AMM</h1>
          <WalletMultiButton />
        </div>

        {connected ? (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <Link href="/swap">
              <div className="bg-purple-900 p-6 rounded-lg hover:bg-purple-800 transition cursor-pointer">
                <h2 className="text-xl font-semibold mb-2">Swap Tokens</h2>
                <p className="text-gray-300">Exchange tokens at the best rates</p>
              </div>
            </Link>
            <Link href="/liquidity">
              <div className="bg-purple-900 p-6 rounded-lg hover:bg-purple-800 transition cursor-pointer">
                <h2 className="text-xl font-semibold mb-2">Manage Liquidity</h2>
                <p className="text-gray-300">Provide liquidity and earn fees</p>
              </div>
            </Link>
          </div>
        ) : (
          <div className="text-center py-12">
            <h2 className="text-2xl font-semibold mb-4">
              Connect your wallet to get started
            </h2>
            <p className="text-gray-300">
              You need to connect a Solana wallet to use the AMM
            </p>
          </div>
        )}
      </div>
    </main>
  );
} 