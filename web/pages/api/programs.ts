import {MongoClient} from 'mongodb';
import {NextApiRequest, NextApiResponse} from 'next';
import {v4 as uuidv4} from 'uuid';

const uri = 'mongodb://root:example@mongo:27017';
const client = new MongoClient(uri);
const dbName = 'sis';
const collectionName = 'schedule';

export default async function handler(
    req: NextApiRequest, res: NextApiResponse) {
  if (req.method === 'GET') {
    try {
      await client.connect();
      const db = client.db(dbName);
      const collection = db.collection(collectionName);

      const programs = await collection.find({}).toArray();
      res.status(200).json(programs);
    } catch (error) {
      console.error('Error fetching programs:', error);
      res.status(500).json({error: 'Failed to fetch programs'});
    } finally {
      await client.close();
    }
  } else if (req.method === 'POST') {
    try {
      const newProgram = {...req.body, id: uuidv4(), version: 1};
      await client.connect();
      const db = client.db(dbName);
      const collection = db.collection(collectionName);

      await collection.insertOne(newProgram);
      res.status(201).json(newProgram);
    } catch (error) {
      console.error('Error creating program:', error);
      res.status(500).json({error: 'Failed to create program'});
    } finally {
      await client.close();
    }
  } else {
    res.setHeader('Allow', ['GET', 'POST']);
    res.status(405).end(`Method ${req.method} Not Allowed`);
  }
}
