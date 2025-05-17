import { NextApiRequest, NextApiResponse } from 'next';
import { MongoClient } from 'mongodb';

const uri = 'mongodb://root:example@mongo:27017';
const client = new MongoClient(uri);
const dbName = 'sis';
const collectionName = 'board_status';

export default async function handler(req: NextApiRequest, res: NextApiResponse) {
  if (req.method === 'GET') {
    try {
      await client.connect();
      const db = client.db(dbName);
      const collection = db.collection(collectionName);

      const statuses = await collection.find({}).toArray();
      res.status(200).json(statuses);
    } catch (error) {
      console.error('Error fetching device statuses:', error);
      res.status(500).json({ error: 'Failed to fetch device statuses' });
    } finally {
      await client.close();
    }
  } else {
    res.setHeader('Allow', ['GET']);
    res.status(405).end(`Method ${req.method} Not Allowed`);
  }
}
