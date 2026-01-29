import Link from 'next/link';

export default function Dashboard() {
  return (
    <div className="p-8">
      <h1 className="text-3xl font-bold mb-4">Nestera Dashboard</h1>
      <p className="mb-6">Manage your active savings groups and track your contributions.</p>
      
      <nav className="flex gap-4 border-t pt-4">
        <Link href="/" className="text-blue-600 hover:underline">← Home</Link>
        <Link href="/savings" className="text-blue-600 hover:underline">Savings Plans →</Link>
      </nav>
    </div>
  );
}