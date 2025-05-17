import {MongoClient} from 'mongodb';
import {NextApiRequest, NextApiResponse} from 'next';

const uri = 'mongodb://root:example@mongo:27017';
const client = new MongoClient(uri);
const dbName = 'sis';
const collectionName = 'schedule';

export default async function handler(
    req: NextApiRequest, res: NextApiResponse) {
  const {id} = req.query;

  if (req.method === 'DELETE') {
    try {
      await client.connect();
      const db = client.db(dbName);
      const collection = db.collection(collectionName);

      await collection.deleteOne({id});
      res.status(200).json({message: 'Program deleted successfully'});
    } catch (error) {
      console.error('Error deleting program:', error);
      res.status(500).json({error: 'Failed to delete program'});
    } finally {
      await client.close();
    }
  } else if (req.method === 'PUT') {
    try {
      const updatedProgram = req.body;
      delete updatedProgram._id;  // Exclude the immutable _id field
      await client.connect();
      const db = client.db(dbName);
      const collection = db.collection(collectionName);

      await collection.updateOne({id}, {$set: updatedProgram});
      await collection.updateOne({id}, {$inc: {version: 1}});
      const updatedProgramFromDb = await collection.findOne({id});
      res.status(200).json(updatedProgramFromDb);
    } catch (error) {
      console.error('Error updating program:', error);
      res.status(500).json({error: 'Failed to update program'});
    } finally {
      await client.close();
    }
  } else {
    res.setHeader('Allow', ['DELETE', 'PUT']);
    res.status(405).end(`Method ${req.method} Not Allowed`);
  }
}
