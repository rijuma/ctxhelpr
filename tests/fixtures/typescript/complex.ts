/** Base repository interface */
interface Repository<T> {
  findById(id: string): Promise<T | null>;
  findAll(): Promise<T[]>;
  save(entity: T): Promise<void>;
  delete(id: string): Promise<void>;
}

/** A user entity */
interface User {
  id: string;
  name: string;
  email: string;
}

/** User repository with caching */
class UserRepository implements Repository<User> {
  private cache: Map<string, User>;
  private db: Database;

  constructor(db: Database) {
    this.db = db;
    this.cache = new Map();
  }

  async findById(id: string): Promise<User | null> {
    const cached = this.cache.get(id);
    if (cached) return cached;

    const user = await this.db.query("SELECT * FROM users WHERE id = ?", id);
    if (user) this.cache.set(id, user);
    return user;
  }

  async findAll(): Promise<User[]> {
    return this.db.query("SELECT * FROM users");
  }

  async save(user: User): Promise<void> {
    await this.db.execute("INSERT INTO users VALUES (?, ?, ?)", user.id, user.name, user.email);
    this.cache.set(user.id, user);
  }

  async delete(id: string): Promise<void> {
    await this.db.execute("DELETE FROM users WHERE id = ?", id);
    this.cache.delete(id);
  }
}

class AdminUserRepository extends UserRepository {
  async findAdmins(): Promise<User[]> {
    return this.findAll();
  }
}
