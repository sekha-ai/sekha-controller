export class SekhaError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'SekhaError';
  }
}

export class SekhaNotFoundError extends SekhaError {
  constructor(message: string = 'Resource not found') {
    super(message);
    this.name = 'SekhaNotFoundError';
  }
}

export class SekhaValidationError extends SekhaError {
  constructor(message: string, public details?: string) {
    super(message);
    this.name = 'SekhaValidationError';
  }
}

export class SekhaAPIError extends SekhaError {
  constructor(message: string, public statusCode: number, public response?: string) {
    super(message);
    this.name = 'SekhaAPIError';
  }
}