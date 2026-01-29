import Link from 'next/link';

export default function HomePage() {
  return (
    <div className="p-8">
      <h1 className="text-3xl font-bold mb-4">Welcome to Nestera</h1>
      <p className="mb-6">Collaborative group savings on the Stellar network.</p>
      <nav className="flex gap-4">
        <Link href="/dashboard" className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">Go to Dashboard</Link>
        <Link href="/savings" className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700">View Savings Plans</Link>
      </nav>
    </div>
  );
}