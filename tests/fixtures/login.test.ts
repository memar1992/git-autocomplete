import { validateLogin } from './login';
console.assert(validateLogin('hello'));
console.assert(!validateLogin(''));
